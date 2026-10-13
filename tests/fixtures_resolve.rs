use std::collections::BTreeMap;
use std::path::Path;

use blast::resolve::{Project, Resolution};

fn fixture(name: &str) -> Project {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    Project::load(&root).unwrap()
}

/// module name -> sorted internal imports
fn edges(p: &Project) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for (id, m) in p.modules.iter().enumerate() {
        let mut deps = Vec::new();
        for imp in &m.parsed.imports {
            if let Resolution::Internal { modules } = p.resolve(id, imp) {
                deps.extend(modules.into_iter().map(|t| p.modules[t].name.clone()));
            }
        }
        deps.sort();
        deps.dedup();
        out.insert(m.name.clone(), deps);
    }
    out
}

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn flask_src_layout() {
    let p = fixture("flaskish");
    let e = edges(&p);

    assert_eq!(
        e["flask.app"],
        v(&[
            "flask",
            "flask.globals",
            "flask.json",
            "flask.sansio.app",
            "flask.testing"
        ])
    );
    // sansio/ has no __init__.py but still lives inside flask
    assert_eq!(e["flask.sansio.app"], v(&["flask", "flask.globals"]));
    assert_eq!(e["flask.json.tag"], v(&["flask", "flask.globals"]));
    assert_eq!(
        e["test_json"],
        v(&["flask", "flask.json", "flask.json.tag"])
    );
    assert_eq!(e["conftest"], v(&["flask"]));
}

#[test]
fn requests_flat_layout() {
    let p = fixture("requestsish");
    let e = edges(&p);

    assert_eq!(e["requests.api"], v(&["requests", "requests.sessions"]));
    assert_eq!(
        e["tests.test_utils"],
        v(&["requests", "requests.utils", "tests", "tests.utils"])
    );

    let compat = p.lookup("requests.compat")[0];
    assert_eq!(p.modules[compat].parsed.dynamic_imports, vec![5]);
}
