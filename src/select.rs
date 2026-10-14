//! Picks the tests a set of changes can reach.
//!
//! The rule everywhere in here: if we aren't sure, run more tests, never fewer.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::diff::{Change, ChangeKind};
use crate::graph::Graph;
use crate::resolve::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Run `selected_tests` only.
    Selected,
    /// Run everything. `reasons` says why.
    Full,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Don't force-select tests that can reach a non-literal dynamic import.
    /// Off by default; strings that name our modules are still followed either way.
    pub trust_dynamic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Selection {
    pub mode: Mode,
    pub reasons: Vec<String>,
    pub changed: Vec<Change>,
    pub affected_modules: Vec<String>,
    pub selected_tests: Vec<PathBuf>,
    pub total_tests: usize,
}

const CONFIG_FILES: &[&str] = &[
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "pytest.ini",
    "tox.ini",
    "noxfile.py",
    "Pipfile",
    "Pipfile.lock",
    "poetry.lock",
    "uv.lock",
    "pdm.lock",
    "Makefile",
    ".coveragerc",
];

const DOC_EXTENSIONS: &[&str] = &["md", "rst", "txt", "png", "jpg", "jpeg", "gif", "svg"];

const IGNORED_FILES: &[&str] = &[
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".readthedocs.yaml",
    ".readthedocs.yml",
    ".pre-commit-config.yaml",
    "CODEOWNERS",
    "mkdocs.yml",
    "mkdocs.yaml",
    "CHANGELOG",
    "AUTHORS",
];

/// `requirements.txt`, `requirements/dev.in`, `requirements-test.txt`, but not
/// `docs/requirements.rst`.
fn is_requirements(path: &Path) -> bool {
    if path.starts_with("docs") {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let in_req_dir = path.parent().is_some_and(|d| d.ends_with("requirements"));
    (name.starts_with("requirements") || in_req_dir) && matches!(ext, "txt" | "in" | "")
}

fn mentions_tests(yaml: &str) -> bool {
    let y = yaml.to_ascii_lowercase();
    ["test", "tox", "nox", "coverage"]
        .iter()
        .any(|w| y.contains(w))
}

/// pytest's default `python_files`: `test_*.py` and `*_test.py`.
pub fn is_test_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with(".py") && (name.starts_with("test_") || name.ends_with("_test.py"))
}

pub fn select(project: &Project, graph: &Graph, changes: &[Change], opts: &Options) -> Selection {
    let mut reasons = Vec::new();
    let mut force_full = false;
    let mut changed_modules = Vec::new();

    for change in changes {
        for path in change.paths() {
            match classify(project, change, path) {
                Kind::Module(id) => changed_modules.push(id),
                Kind::Ignore => {}
                Kind::Full(why) => {
                    force_full = true;
                    reasons.push(why);
                }
            }
        }
    }

    let affected = graph.dependents(&changed_modules);
    let library_changed = changed_modules
        .iter()
        .any(|&id| !is_test_file(&project.modules[id].path));

    // modules that could import anything: test reaching them is always in
    let mut wildcards = BTreeSet::new();
    for (id, m) in project.modules.iter().enumerate() {
        if !opts.trust_dynamic && !m.parsed.dynamic_imports.is_empty() {
            wildcards.insert(id);
        }
        if !graph.unresolved[id].is_empty() {
            wildcards.insert(id);
        }
    }

    let tests: Vec<usize> = (0..project.modules.len())
        .filter(|&id| is_test_file(&project.modules[id].path))
        .collect();

    let mut selected = Vec::new();
    let mut wildcard_hits = BTreeSet::new();
    for &t in &tests {
        let reach = test_closure(project, graph, t);
        let hits_change = changed_modules.iter().any(|c| reach.contains(c));
        let hit_wildcards: Vec<usize> = reach.intersection(&wildcards).copied().collect();
        // a wildcard can only pull in code that changed, and nothing dynamically imports
        // a test file, so a docs- or tests-only diff doesn't trigger it
        let hits_wildcard = !hit_wildcards.is_empty() && library_changed;
        if hits_change || hits_wildcard {
            selected.push(project.modules[t].path.clone());
        }
        if !hits_change && hits_wildcard {
            wildcard_hits.extend(hit_wildcards);
        }
    }

    for id in wildcard_hits {
        let m = &project.modules[id];
        if let Some(line) = m.parsed.dynamic_imports.first() {
            reasons.push(format!(
                "{}:{line} has a dynamic import, running tests that reach it",
                m.path.display()
            ));
        }
        if let Some(why) = graph.unresolved[id].first() {
            reasons.push(format!(
                "{}: {why}, running tests that reach it",
                m.path.display()
            ));
        }
    }

    let mode = if force_full || (!tests.is_empty() && selected.len() == tests.len()) {
        Mode::Full
    } else {
        Mode::Selected
    };
    if mode == Mode::Full && !force_full {
        reasons.push("every test can reach the change".to_string());
    }
    if mode == Mode::Full {
        selected = tests
            .iter()
            .map(|&t| project.modules[t].path.clone())
            .collect();
    }

    Selection {
        mode,
        reasons,
        changed: changes.to_vec(),
        affected_modules: affected
            .into_iter()
            .map(|id| project.modules[id].name.clone())
            .collect(),
        selected_tests: selected,
        total_tests: tests.len(),
    }
}

/// Everything a test can reach: its own imports plus those of every `conftest.py`
/// above it, which pytest loads implicitly.
fn test_closure(project: &Project, graph: &Graph, test: usize) -> BTreeSet<usize> {
    let mut reach = graph.dependencies(test);
    let mut dir = project.modules[test].path.parent();
    while let Some(d) = dir {
        if let Some(c) = project.module_by_path(&d.join("conftest.py")) {
            reach.extend(graph.dependencies(c));
        }
        dir = d.parent();
    }
    reach
}

enum Kind {
    Module(usize),
    Ignore,
    Full(String),
}

fn classify(project: &Project, change: &Change, path: &Path) -> Kind {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let shown = path.display();

    if name == "conftest.py" {
        return Kind::Full(format!("{shown} changed, fixtures can affect any test"));
    }
    if CONFIG_FILES.contains(&name) || is_requirements(path) {
        return Kind::Full(format!("{shown} is build/test config"));
    }
    if path.starts_with(".github/workflows") {
        // lint / codeql / release workflows don't change test results
        return match std::fs::read_to_string(project.root.join(path)) {
            Ok(yaml) if !mentions_tests(&yaml) => Kind::Ignore,
            _ => Kind::Full(format!("{shown} changes how CI runs tests")),
        };
    }

    if ext == "py" {
        if let Some(id) = project.module_by_path(path) {
            return Kind::Module(id);
        }
        let gone = change.kind == ChangeKind::Deleted
            || (change.kind == ChangeKind::Renamed && change.old_path.as_deref() == Some(path));
        if gone {
            // whatever imported it was either edited in this change (and so is picked up
            // anyway) or now fails to resolve. `from pkg import gone` still resolves to
            // `pkg` though, so treat the parent package as changed too.
            return match path
                .parent()
                .and_then(|d| project.module_by_path(&d.join("__init__.py")))
            {
                Some(pkg) => Kind::Module(pkg),
                None => Kind::Full(format!(
                    "{shown} was removed and isn't inside a package, can't tell what used it"
                )),
            };
        }
        if path.starts_with("docs") {
            return Kind::Ignore;
        }
        return Kind::Full(format!(
            "{shown} isn't an importable module, might be run by tests"
        ));
    }

    if DOC_EXTENSIONS.contains(&ext) && !name.starts_with("requirements") {
        return Kind::Ignore;
    }
    if IGNORED_FILES.contains(&name)
        || path.starts_with("docs")
        || path.starts_with(".github")
        || name.starts_with("LICENSE")
    {
        return Kind::Ignore;
    }

    Kind::Full(format!("{shown} isn't Python, tests might read it"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_files() {
        assert!(is_requirements(Path::new("requirements.txt")));
        assert!(is_requirements(Path::new("requirements-dev.txt")));
        assert!(is_requirements(Path::new("requirements/tests.in")));
        assert!(!is_requirements(Path::new("docs/requirements.txt")));
        assert!(!is_requirements(Path::new("docs/requirements.rst")));
        assert!(!is_requirements(Path::new("requirements.md")));
    }

    #[test]
    fn workflow_sniffing() {
        assert!(mentions_tests("jobs:\n  tests:\n    run: pytest\n"));
        assert!(mentions_tests("run: tox -e py312\n"));
        assert!(!mentions_tests(
            "name: CodeQL\njobs:\n  analyze:\n    uses: github/codeql-action\n"
        ));
    }

    #[test]
    fn test_file_patterns() {
        assert!(is_test_file(Path::new("tests/test_app.py")));
        assert!(is_test_file(Path::new("pkg/app_test.py")));
        assert!(!is_test_file(Path::new("tests/conftest.py")));
        assert!(!is_test_file(Path::new("tests/helpers.py")));
        assert!(!is_test_file(Path::new("tests/test_data.json")));
    }
}
