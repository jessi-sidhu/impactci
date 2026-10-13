//! Changed files between two git revisions.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use git2::{Delta, DiffFindOptions, DiffOptions, Repository};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
    pub kind: ChangeKind,
}

impl Change {
    /// Every path this change touches (both sides of a rename).
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.path.as_path()).chain(self.old_path.as_deref())
    }
}

/// Files changed in `range`, which is `base..head`, `base...head` or just `base`
/// (meaning `base..HEAD`). Like a GitHub PR, we always diff from the merge base,
/// so commits that landed on `base` after the branch point don't show up.
pub fn changed_files(repo_path: &Path, range: &str) -> Result<Vec<Change>> {
    let repo = Repository::discover(repo_path)
        .with_context(|| format!("no git repo at {}", repo_path.display()))?;

    let (base, head) = match range.split_once("...").or_else(|| range.split_once("..")) {
        Some((b, h)) => (b, if h.is_empty() { "HEAD" } else { h }),
        None => (range, "HEAD"),
    };
    if base.is_empty() {
        bail!("bad range `{range}`, expected something like main..HEAD");
    }

    let base_commit = repo
        .revparse_single(base)
        .with_context(|| format!("can't find revision `{base}`"))?
        .peel_to_commit()?;
    let head_commit = repo
        .revparse_single(head)
        .with_context(|| format!("can't find revision `{head}`"))?
        .peel_to_commit()?;

    let merge_base = repo
        .merge_base(base_commit.id(), head_commit.id())
        .with_context(|| format!("`{base}` and `{head}` have no common ancestor"))?;
    let old_tree = repo.find_commit(merge_base)?.tree()?;
    let new_tree = head_commit.tree()?;

    let mut opts = DiffOptions::new();
    let mut diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;

    let mut out = Vec::new();
    for delta in diff.deltas() {
        let new = delta.new_file().path().map(Path::to_path_buf);
        let old = delta.old_file().path().map(Path::to_path_buf);
        let change = match delta.status() {
            Delta::Added | Delta::Copied => Change {
                path: new.context("added file without a path")?,
                old_path: None,
                kind: ChangeKind::Added,
            },
            Delta::Deleted => Change {
                path: old.context("deleted file without a path")?,
                old_path: None,
                kind: ChangeKind::Deleted,
            },
            Delta::Renamed => Change {
                path: new.context("renamed file without a path")?,
                old_path: old,
                kind: ChangeKind::Renamed,
            },
            // modified, type change (file <-> symlink), anything odd: treat as modified
            _ => Change {
                path: new.or(old).context("change without a path")?,
                old_path: None,
                kind: ChangeKind::Modified,
            },
        };
        out.push(change);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}
