//! Markdown for the PR comment.

use std::fmt::Write;

use crate::risk::Risk;
use crate::select::{Mode, Selection};

/// Hidden marker so the action can find and update its own comment.
pub const MARKER: &str = "<!-- blast-report -->";

const MAX_LISTED: usize = 30;

pub fn markdown(sel: &Selection, risks: &[Risk]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{MARKER}");

    let headline = match sel.mode {
        Mode::Full => format!("running the full suite ({} test files)", sel.total_tests),
        Mode::Selected if sel.selected_tests.is_empty() => "no tests affected".to_string(),
        Mode::Selected => format!(
            "running {} of {} test files",
            sel.selected_tests.len(),
            sel.total_tests
        ),
    };
    let _ = writeln!(s, "### blast radius: {headline}\n");

    let py_changed = sel
        .changed
        .iter()
        .filter(|c| c.path.extension().is_some_and(|e| e == "py"))
        .count();
    let _ = writeln!(
        s,
        "{} files changed ({} Python) → {} modules affected\n",
        sel.changed.len(),
        py_changed,
        sel.affected_modules.len()
    );

    if !sel.reasons.is_empty() {
        let label = match sel.mode {
            Mode::Full => "Why the full suite",
            Mode::Selected => "Also included",
        };
        let _ = writeln!(s, "**{label}:**\n");
        for r in &sel.reasons {
            let _ = writeln!(s, "- {r}");
        }
        s.push('\n');
    }

    if !risks.is_empty() {
        let _ = writeln!(s, "**Riskiest changes**\n");
        let _ = writeln!(
            s,
            "| file | modules depending on it | commits in last 300 |"
        );
        let _ = writeln!(s, "|---|---|---|");
        for r in risks.iter().take(5) {
            let _ = writeln!(
                s,
                "| `{}` | {} | {} |",
                r.path.display(),
                r.dependents,
                r.recent_commits
            );
        }
        s.push('\n');
    }

    if sel.mode == Mode::Selected && !sel.selected_tests.is_empty() {
        let items: Vec<String> = sel
            .selected_tests
            .iter()
            .map(|t| format!("`{}`", t.display()))
            .collect();
        details(&mut s, "Selected tests", &items);
    }
    if !sel.affected_modules.is_empty() {
        let items: Vec<String> = sel
            .affected_modules
            .iter()
            .map(|m| format!("`{m}`"))
            .collect();
        details(&mut s, "Affected modules", &items);
    }

    let _ = writeln!(
        s,
        "<sub>static import analysis by blast · when unsure it runs more tests, never fewer</sub>"
    );
    s
}

fn details(s: &mut String, title: &str, items: &[String]) {
    let _ = writeln!(s, "<details><summary>{title} ({})</summary>\n", items.len());
    for i in items.iter().take(MAX_LISTED) {
        let _ = writeln!(s, "- {i}");
    }
    if items.len() > MAX_LISTED {
        let _ = writeln!(s, "- …and {} more", items.len() - MAX_LISTED);
    }
    let _ = writeln!(s, "\n</details>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{Change, ChangeKind};

    fn sel(mode: Mode, tests: &[&str], reasons: &[&str]) -> Selection {
        Selection {
            mode,
            reasons: reasons.iter().map(|r| r.to_string()).collect(),
            changed: vec![Change {
                path: "app/models.py".into(),
                old_path: None,
                kind: ChangeKind::Modified,
            }],
            affected_modules: vec!["app.models".into(), "app.views".into()],
            selected_tests: tests.iter().map(Into::into).collect(),
            total_tests: 10,
        }
    }

    #[test]
    fn selected_report() {
        let md = markdown(
            &sel(Mode::Selected, &["tests/test_models.py"], &[]),
            &[Risk {
                path: "app/models.py".into(),
                dependents: 4,
                recent_commits: 7,
                score: 1.0,
            }],
        );
        assert!(md.starts_with(MARKER));
        assert!(md.contains("running 1 of 10 test files"));
        assert!(md.contains("| `app/models.py` | 4 | 7 |"));
        assert!(md.contains("<summary>Selected tests (1)</summary>"));
    }

    #[test]
    fn full_report_lists_reasons() {
        let md = markdown(
            &sel(Mode::Full, &[], &["pyproject.toml is build/test config"]),
            &[],
        );
        assert!(md.contains("running the full suite"));
        assert!(md.contains("**Why the full suite:**"));
        assert!(!md.contains("Selected tests"));
    }

    #[test]
    fn nothing_affected() {
        let md = markdown(&sel(Mode::Selected, &[], &[]), &[]);
        assert!(md.contains("no tests affected"));
    }
}
