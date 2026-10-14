//! The bits of pytest config we care about: which files count as tests.

use std::fs;
use std::path::Path;

pub const DEFAULT_PYTHON_FILES: &[&str] = &["test_*.py", "*_test.py"];

/// `python_files` from pytest.ini, pyproject.toml, tox.ini or setup.cfg (same order
/// pytest checks them), or pytest's defaults.
pub fn python_files(root: &Path) -> Vec<String> {
    let candidates = [
        ("pytest.ini", "[pytest]"),
        ("pyproject.toml", "[tool.pytest.ini_options]"),
        ("tox.ini", "[pytest]"),
        ("setup.cfg", "[tool:pytest]"),
    ];
    for (file, section) in candidates {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        if let Some(found) = find_python_files(&text, section) {
            return found;
        }
    }
    DEFAULT_PYTHON_FILES.iter().map(|s| s.to_string()).collect()
}

fn find_python_files(text: &str, section: &str) -> Option<Vec<String>> {
    let mut in_section = false;
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "python_files" {
            continue;
        }
        let mut value = value.trim().to_string();
        // toml arrays can span lines; ini values can continue on indented lines
        if value.starts_with('[') {
            while !value.contains(']') {
                match lines.next() {
                    Some(l) => value.push_str(l),
                    None => break,
                }
            }
        } else {
            while let Some(next) = lines.peek() {
                if next.starts_with([' ', '\t']) && !next.trim().is_empty() {
                    value.push(' ');
                    value.push_str(next.trim());
                    lines.next();
                } else {
                    break;
                }
            }
        }
        let patterns: Vec<String> = value
            .split(|c: char| c.is_whitespace() || c == ',')
            .map(|p| p.trim_matches(|c| matches!(c, '[' | ']' | '"' | '\'')))
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect();
        if !patterns.is_empty() {
            return Some(patterns);
        }
    }
    None
}

/// Glob match on a file name, `*` and `?` only (that's all pytest patterns use).
pub fn matches(pattern: &str, name: &str) -> bool {
    fn go(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => go(&p[1..], n) || (!n.is_empty() && go(p, &n[1..])),
            (Some(b'?'), Some(_)) => go(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a == b => go(&p[1..], &n[1..]),
            _ => false,
        }
    }
    go(pattern.as_bytes(), name.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globbing() {
        assert!(matches("test_*.py", "test_app.py"));
        assert!(matches("*_test.py", "app_test.py"));
        assert!(matches("tests.py", "tests.py"));
        assert!(matches("check_?.py", "check_a.py"));
        assert!(!matches("test_*.py", "conftest.py"));
        assert!(!matches("test_*.py", "test_app.pyc"));
    }

    #[test]
    fn toml_array() {
        let t = "[project]\nname='x'\n\n[tool.pytest.ini_options]\npython_files = [\n  \"tests.py\",\n  \"test_*.py\",\n]\n";
        assert_eq!(
            find_python_files(t, "[tool.pytest.ini_options]"),
            Some(vec!["tests.py".into(), "test_*.py".into()])
        );
    }

    #[test]
    fn ini_space_separated() {
        let t = "[pytest]\naddopts = -q\npython_files = tests.py test_*.py\n    *_tests.py\n";
        assert_eq!(
            find_python_files(t, "[pytest]"),
            Some(vec![
                "tests.py".into(),
                "test_*.py".into(),
                "*_tests.py".into()
            ])
        );
    }

    #[test]
    fn other_sections_ignored() {
        let t = "[flake8]\npython_files = nope.py\n[tool:pytest]\naddopts = -q\n";
        assert_eq!(find_python_files(t, "[tool:pytest]"), None);
    }
}
