//! Finds the Python modules in a repo and resolves imports to them.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;

use crate::parser::{self, Import, ParsedFile};

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".tox",
    ".nox",
    ".venv",
    "venv",
    "env",
    "node_modules",
    "__pycache__",
    "build",
    "dist",
    "site-packages",
    ".mypy_cache",
    ".pytest_cache",
];

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    /// Canonical dotted name, e.g. `flask.json.tag`.
    pub name: String,
    /// Path relative to the project root.
    pub path: PathBuf,
    pub is_package: bool,
    #[serde(skip)]
    pub parsed: ParsedFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resolution {
    /// Points at one or more modules in this repo (the target plus parent packages).
    /// `parents` is the subset that only gets pulled in because Python runs a package's
    /// `__init__.py` before its submodules, e.g. `pkg` in `from pkg.mod import x`.
    Internal {
        modules: Vec<usize>,
        parents: Vec<usize>,
    },
    /// Third-party or stdlib. Ignored for selection.
    External,
    /// Looks like it should be in this repo but we couldn't find it.
    Unresolved { reason: String },
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub modules: Vec<Module>,
    /// Every dotted name a module can be reached by. Usually one, sometimes a
    /// root-relative alias too (tests dirs without `__init__.py`).
    by_name: HashMap<String, Vec<usize>>,
    by_path: HashMap<PathBuf, usize>,
    top_level: BTreeSet<String>,
}

impl Project {
    pub fn load(root: &Path) -> Result<Self> {
        let root = root.to_path_buf();
        let mut modules = Vec::new();

        let walker = WalkDir::new(&root).into_iter().filter_entry(|e| {
            if e.depth() == 0 || !e.file_type().is_dir() {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !(SKIP_DIRS.contains(&name.as_ref()) || name.starts_with('.'))
        });

        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "py") {
                continue;
            }
            let rel = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
            let Some(name) = package_name(&root, &rel) else {
                continue;
            };
            let src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue, // not utf-8, skip it
            };
            let parsed = parser::parse_source(&src)?;
            modules.push(Module {
                name,
                is_package: rel.file_name().is_some_and(|f| f == "__init__.py"),
                path: rel,
                parsed,
            });
        }
        modules.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Self::from_modules(root, modules))
    }

    pub fn from_modules(root: PathBuf, modules: Vec<Module>) -> Self {
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_path = HashMap::new();
        let mut top_level = BTreeSet::new();

        for (id, m) in modules.iter().enumerate() {
            by_path.insert(m.path.clone(), id);
            let mut names = vec![m.name.clone()];
            if let Some(alias) = root_relative_name(&m.path)
                && alias != m.name
            {
                names.push(alias);
            }
            for n in names {
                top_level.insert(n.split('.').next().unwrap_or(&n).to_string());
                by_name.entry(n).or_default().push(id);
            }
        }

        Project {
            root,
            modules,
            by_name,
            by_path,
            top_level,
        }
    }

    pub fn module_by_path(&self, rel: &Path) -> Option<usize> {
        self.by_path.get(rel).copied()
    }

    pub fn lookup(&self, name: &str) -> &[usize] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Resolve one import made by module `from`.
    pub fn resolve(&self, from: usize, imp: &Import) -> Resolution {
        let base = if imp.level == 0 {
            imp.module.clone()
        } else {
            let me = &self.modules[from];
            let mut parts: Vec<&str> = me.name.split('.').collect();
            if !me.is_package {
                parts.pop();
            }
            // `from . import x` is level 1 and stays in our own package
            let ups = imp.level - 1;
            if parts.len() <= ups {
                return Resolution::Unresolved {
                    reason: format!(
                        "relative import goes above the top-level package (line {})",
                        imp.line
                    ),
                };
            }
            parts.truncate(parts.len() - ups);
            let mut s = parts.join(".");
            if !imp.module.is_empty() {
                if !s.is_empty() {
                    s.push('.');
                }
                s.push_str(&imp.module);
            }
            s
        };

        if base.is_empty() {
            return Resolution::Unresolved {
                reason: format!("empty relative import (line {})", imp.line),
            };
        }

        let top = base.split('.').next().unwrap_or("");
        if imp.level == 0 && !self.top_level.contains(top) {
            return Resolution::External;
        }

        let mut explicit = BTreeSet::new();
        let mut implicit = BTreeSet::new();

        // the module itself, or for `from pkg import sub` the submodules
        let base_hits = self.lookup(&base);
        let mut all_names_are_submodules = !imp.names.is_empty();
        for name in &imp.names {
            let hits = if name == "*" {
                &[][..]
            } else {
                self.lookup(&format!("{base}.{name}"))
            };
            all_names_are_submodules &= !hits.is_empty();
            explicit.extend(hits.iter().copied());
        }

        if base_hits.is_empty() && explicit.is_empty() {
            return Resolution::Unresolved {
                reason: format!("can't find module `{base}` (line {})", imp.line),
            };
        }

        // `from pkg import sub` only runs pkg/__init__.py, it doesn't use anything from it
        if all_names_are_submodules {
            implicit.extend(base_hits.iter().copied());
        } else {
            explicit.extend(base_hits.iter().copied());
        }

        // importing a.b.c runs a/__init__.py and a/b/__init__.py first. A plain
        // `import a.b.c` also binds `a`, so there it's a real use.
        let parts: Vec<&str> = base.split('.').collect();
        for i in 1..parts.len() {
            let hits = self.lookup(&parts[..i].join("."));
            if imp.names.is_empty() {
                explicit.extend(hits.iter().copied());
            } else {
                implicit.extend(hits.iter().copied());
            }
        }
        explicit.remove(&from);
        implicit.remove(&from);
        let parents: Vec<usize> = implicit.difference(&explicit).copied().collect();
        explicit.extend(implicit);

        Resolution::Internal {
            modules: explicit.into_iter().collect(),
            parents,
        }
    }
}

/// Dotted name based on `__init__.py` packages: climb while the parent dir is a package.
/// `src/flask/json/tag.py` -> `flask.json.tag`, `tests/test_x.py` -> `test_x`.
///
/// A dir without `__init__.py` sitting inside a real package (flask's `sansio/`) is a
/// namespace subpackage, so we keep climbing through it.
fn package_name(root: &Path, rel: &Path) -> Option<String> {
    let is_pkg = |d: &Path| !d.as_os_str().is_empty() && root.join(d).join("__init__.py").is_file();

    let stem = rel.file_stem()?.to_string_lossy().to_string();
    let mut parts = Vec::new();
    if stem != "__init__" {
        parts.push(stem);
    }
    let mut dir = rel.parent();
    while let Some(d) = dir {
        if d.as_os_str().is_empty() || !(is_pkg(d) || d.parent().is_some_and(is_pkg)) {
            break;
        }
        parts.push(d.file_name()?.to_string_lossy().to_string());
        dir = d.parent();
    }
    if parts.is_empty() {
        return None;
    }
    parts.reverse();
    let name = parts.join(".");
    if name.split('.').all(is_identifier) {
        Some(name)
    } else {
        None
    }
}

/// Name relative to the repo root (or `src/`), for namespace-style imports like `tests.helpers`.
fn root_relative_name(rel: &Path) -> Option<String> {
    let rel = rel.strip_prefix("src").unwrap_or(rel);
    let mut parts: Vec<String> = rel
        .parent()?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let stem = rel.file_stem()?.to_string_lossy().to_string();
    if stem != "__init__" {
        parts.push(stem);
    }
    if parts.is_empty() || !parts.iter().all(|p| is_identifier(p)) {
        return None;
    }
    Some(parts.join("."))
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, path: &str, src: &str) -> Module {
        Module {
            name: name.into(),
            path: path.into(),
            is_package: path.ends_with("__init__.py"),
            parsed: parser::parse_source(src).unwrap(),
        }
    }

    fn project() -> Project {
        Project::from_modules(
            "/repo".into(),
            vec![
                module("flask", "src/flask/__init__.py", "from .app import Flask\n"),
                module(
                    "flask.app",
                    "src/flask/app.py",
                    "from . import json\nfrom .json.tag import T\nimport click\n",
                ),
                module("flask.json", "src/flask/json/__init__.py", ""),
                module(
                    "flask.json.tag",
                    "src/flask/json/tag.py",
                    "from ..globals import g\n",
                ),
                module(
                    "flask.globals",
                    "src/flask/globals.py",
                    "from ...nope import x\n",
                ),
                module(
                    "test_app",
                    "tests/test_app.py",
                    "import flask\nfrom flask.missing import y\n",
                ),
            ],
        )
    }

    fn names(p: &Project, r: &Resolution) -> Vec<String> {
        match r {
            Resolution::Internal { modules, .. } => {
                modules.iter().map(|&i| p.modules[i].name.clone()).collect()
            }
            other => panic!("expected internal, got {other:?}"),
        }
    }

    fn resolve(p: &Project, from: usize, i: usize) -> Resolution {
        p.resolve(from, &p.modules[from].parsed.imports[i])
    }

    #[test]
    fn relative_from_package_init() {
        let p = project();
        assert_eq!(names(&p, &resolve(&p, 0, 0)), vec!["flask.app"]);
    }

    #[test]
    fn from_dot_import_submodule() {
        let p = project();
        assert_eq!(names(&p, &resolve(&p, 1, 0)), vec!["flask", "flask.json"]);
    }

    #[test]
    fn includes_parent_packages() {
        let p = project();
        assert_eq!(
            names(&p, &resolve(&p, 1, 1)),
            vec!["flask", "flask.json", "flask.json.tag"]
        );
    }

    #[test]
    fn two_dots() {
        let p = project();
        assert_eq!(
            names(&p, &resolve(&p, 3, 0)),
            vec!["flask", "flask.globals"]
        );
    }

    fn parents(p: &Project, r: &Resolution) -> Vec<String> {
        match r {
            Resolution::Internal { parents, .. } => {
                parents.iter().map(|&i| p.modules[i].name.clone()).collect()
            }
            other => panic!("expected internal, got {other:?}"),
        }
    }

    #[test]
    fn parent_packages_are_marked() {
        let p = project();
        // from .json.tag import T: flask and flask.json only run, T comes from tag
        assert_eq!(parents(&p, &resolve(&p, 1, 1)), vec!["flask", "flask.json"]);
        // from . import json: json is a submodule, so flask/__init__ is just a parent
        assert_eq!(parents(&p, &resolve(&p, 1, 0)), vec!["flask"]);
        // import flask: a real use
        assert!(parents(&p, &resolve(&p, 5, 0)).is_empty());
    }

    #[test]
    fn third_party_is_external() {
        let p = project();
        assert_eq!(resolve(&p, 1, 2), Resolution::External);
    }

    #[test]
    fn too_many_dots_is_unresolved() {
        let p = project();
        assert!(matches!(resolve(&p, 4, 0), Resolution::Unresolved { .. }));
    }

    #[test]
    fn missing_internal_module_is_unresolved() {
        let p = project();
        assert!(matches!(resolve(&p, 5, 1), Resolution::Unresolved { .. }));
    }

    #[test]
    fn absolute_from_tests() {
        let p = project();
        assert_eq!(names(&p, &resolve(&p, 5, 0)), vec!["flask"]);
    }

    #[test]
    fn identifiers() {
        assert!(is_identifier("_private"));
        assert!(!is_identifier("2fa"));
        assert!(!is_identifier("my-pkg"));
    }
}
