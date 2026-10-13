//! Pulls import statements out of Python source with tree-sitter.

use anyhow::{Context, Result};
use serde::Serialize;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Import {
    /// Dotted module path as written, without leading dots. Empty for `from . import x`.
    pub module: String,
    /// Names pulled in with `from x import a, b`. Empty for plain `import x`. `*` for wildcards.
    pub names: Vec<String>,
    /// Number of leading dots for relative imports, 0 for absolute.
    pub level: usize,
    /// 1-based line number.
    pub line: usize,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ParsedFile {
    pub imports: Vec<Import>,
    /// Lines that call `importlib.import_module`, `__import__` and friends.
    /// We can't follow these statically, so callers should treat them as "could import anything".
    pub dynamic_imports: Vec<usize>,
}

pub fn parse_source(src: &str) -> Result<ParsedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .context("loading python grammar")?;
    let tree = parser
        .parse(src, None)
        .context("tree-sitter failed to parse")?;

    let mut out = ParsedFile::default();
    walk(tree.root_node(), src.as_bytes(), &mut out);
    Ok(out)
}

fn walk(node: Node, src: &[u8], out: &mut ParsedFile) {
    match node.kind() {
        "import_statement" => {
            read_import(node, src, out);
            return;
        }
        "import_from_statement" => {
            read_from_import(node, src, out);
            return;
        }
        "call" if is_dynamic_import(node, src) => {
            out.dynamic_imports.push(line_of(node));
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, out);
    }
}

// import a.b, c as d
fn read_import(node: Node, src: &[u8], out: &mut ParsedFile) {
    let mut cursor = node.walk();
    for name in node.children_by_field_name("name", &mut cursor) {
        if let Some(module) = dotted_name_of(name, src) {
            out.imports.push(Import {
                module,
                names: vec![],
                level: 0,
                line: line_of(node),
            });
        }
    }
}

// from a.b import c, d as e  /  from ..x import *  /  from . import y
fn read_from_import(node: Node, src: &[u8], out: &mut ParsedFile) {
    let Some(module_node) = node.child_by_field_name("module_name") else {
        return;
    };

    let (module, level) = if module_node.kind() == "relative_import" {
        let mut level = 0;
        let mut module = String::new();
        let mut cursor = module_node.walk();
        for child in module_node.children(&mut cursor) {
            match child.kind() {
                "import_prefix" => level = text(child, src).chars().filter(|c| *c == '.').count(),
                "dotted_name" => module = text(child, src).to_string(),
                _ => {}
            }
        }
        (module, level)
    } else {
        (text(module_node, src).to_string(), 0)
    };

    let mut names = Vec::new();
    let mut cursor = node.walk();
    for name in node.children_by_field_name("name", &mut cursor) {
        if let Some(n) = dotted_name_of(name, src) {
            names.push(n);
        }
    }
    let mut cursor = node.walk();
    if node
        .children(&mut cursor)
        .any(|c| c.kind() == "wildcard_import")
    {
        names.push("*".to_string());
    }

    out.imports.push(Import {
        module,
        names,
        level,
        line: line_of(node),
    });
}

/// `a.b` or `a.b as c` -> `a.b`
fn dotted_name_of(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "dotted_name" => Some(strip_ws(text(node, src))),
        "aliased_import" => node
            .child_by_field_name("name")
            .map(|n| strip_ws(text(n, src))),
        _ => None,
    }
}

fn is_dynamic_import(call: Node, src: &[u8]) -> bool {
    let Some(func) = call.child_by_field_name("function") else {
        return false;
    };
    let name = text(func, src);
    matches!(
        name,
        "__import__"
            | "import_module"
            | "importlib.import_module"
            | "importlib.__import__"
            | "import_string"
            | "importlib.util.spec_from_file_location"
            | "spec_from_file_location"
    )
}

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

// tree-sitter keeps whitespace inside things like `a . b`, which is legal python
fn strip_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn line_of(node: Node) -> usize {
    node.start_position().row + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imports(src: &str) -> Vec<(String, Vec<String>, usize)> {
        parse_source(src)
            .unwrap()
            .imports
            .into_iter()
            .map(|i| (i.module, i.names, i.level))
            .collect()
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn plain_imports() {
        assert_eq!(
            imports("import os\nimport a.b.c, d as e\n"),
            vec![
                ("os".into(), vec![], 0),
                ("a.b.c".into(), vec![], 0),
                ("d".into(), vec![], 0),
            ]
        );
    }

    #[test]
    fn from_imports() {
        assert_eq!(
            imports("from a.b import c, d as e\nfrom x import (\n    y,\n    z,\n)\n"),
            vec![
                ("a.b".into(), s(&["c", "d"]), 0),
                ("x".into(), s(&["y", "z"]), 0),
            ]
        );
    }

    #[test]
    fn relative_imports() {
        assert_eq!(
            imports("from . import views\nfrom .globals import g\nfrom ..json.tag import T\n"),
            vec![
                ("".into(), s(&["views"]), 1),
                ("globals".into(), s(&["g"]), 1),
                ("json.tag".into(), s(&["T"]), 2),
            ]
        );
    }

    #[test]
    fn wildcard() {
        assert_eq!(
            imports("from .compat import *\n"),
            vec![("compat".into(), s(&["*"]), 1)]
        );
    }

    #[test]
    fn nested_imports_are_found() {
        let src = r#"
import typing as t
if t.TYPE_CHECKING:
    from .app import Flask

def f():
    try:
        import simplejson
    except ImportError:
        import json
"#;
        let got: Vec<_> = imports(src).into_iter().map(|i| i.0).collect();
        assert_eq!(got, s(&["typing", "app", "simplejson", "json"]));
    }

    #[test]
    fn dynamic_imports_flagged() {
        let src = "import importlib\nm = importlib.import_module('x')\nn = __import__('y')\n";
        let parsed = parse_source(src).unwrap();
        assert_eq!(parsed.dynamic_imports, vec![2, 3]);
    }

    #[test]
    fn future_imports_ignored() {
        assert!(imports("from __future__ import annotations\n").is_empty());
    }

    #[test]
    fn line_numbers() {
        let parsed = parse_source("\n\nimport os\n").unwrap();
        assert_eq!(parsed.imports[0].line, 3);
    }
}
