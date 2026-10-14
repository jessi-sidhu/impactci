//! Rough "how scary is this change" score for the PR comment.
//!
//! Two signals, both cheap:
//! - reach: how many modules transitively depend on the file
//! - churn: how many recent commits touched it (files that change a lot break a lot)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use git2::{DiffOptions, Repository, Sort};
use serde::Serialize;

use crate::graph::Graph;
use crate::resolve::Project;

/// How far back to look for churn.
const CHURN_COMMITS: usize = 300;

#[derive(Debug, Clone, Serialize)]
pub struct Risk {
    pub path: PathBuf,
    pub dependents: usize,
    pub recent_commits: usize,
    pub score: f64,
}

/// Commits touching each file among the last `CHURN_COMMITS` first-parent commits.
/// Paths are relative to the repo root.
pub fn churn(repo_path: &Path) -> Result<HashMap<PathBuf, usize>> {
    let repo = Repository::discover(repo_path)?;
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.simplify_first_parent()?;
    walk.set_sorting(Sort::TOPOLOGICAL)?;

    let mut counts = HashMap::new();
    for oid in walk.take(CHURN_COMMITS) {
        let commit = repo.find_commit(oid?)?;
        let Ok(parent) = commit.parent(0) else {
            continue;
        };
        let diff = repo.diff_tree_to_tree(
            Some(&parent.tree()?),
            Some(&commit.tree()?),
            Some(DiffOptions::new().skip_binary_check(true)),
        )?;
        for delta in diff.deltas() {
            if let Some(p) = delta.new_file().path() {
                *counts.entry(p.to_path_buf()).or_insert(0) += 1;
            }
        }
    }
    Ok(counts)
}

/// Score the changed Python modules, riskiest first.
pub fn score(
    project: &Project,
    graph: &Graph,
    changed: &[PathBuf],
    churn: &HashMap<PathBuf, usize>,
    prefix: &Path,
) -> Vec<Risk> {
    let mut out: Vec<Risk> = changed
        .iter()
        .filter(|p| !project.is_test(p))
        .filter_map(|p| {
            let id = project.module_by_path(p)?;
            let dependents = graph.dependents(&[id]).len() - 1;
            let recent_commits = churn.get(&prefix.join(p)).copied().unwrap_or(0);
            // log so one huge number doesn't drown out the other
            let score = ((1 + dependents) as f64).ln() * ((2 + recent_commits) as f64).ln();
            Some(Risk {
                path: p.clone(),
                dependents,
                recent_commits,
                score: (score * 100.0).round() / 100.0,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.path.cmp(&b.path)));
    out
}
