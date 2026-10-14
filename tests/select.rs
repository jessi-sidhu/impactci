mod common;

use std::path::PathBuf;

use blast::diff::changed_files;
use blast::graph::Graph;
use blast::resolve::Project;
use blast::select::{Mode, Options, Selection, select};
use common::TestRepo;

/// A small app:
///   app/models.py   <- app/views.py <- app/__init__.py imports nothing
///   app/utils.py    (standalone)
///   tests/test_models.py, tests/test_views.py, tests/test_utils.py
fn base_repo() -> (TestRepo, String) {
    let r = TestRepo::new();
    r.write("app/__init__.py", "")
        .write("app/models.py", "class User:\n    pass\n")
        .write("app/views.py", "from .models import User\n")
        .write("app/utils.py", "def slug(s):\n    return s.lower()\n")
        .write("tests/conftest.py", "import pytest\n")
        .write("tests/test_models.py", "from app.models import User\n")
        .write("tests/test_views.py", "from app import views\n")
        .write("tests/test_utils.py", "from app.utils import slug\n")
        .write("README.md", "# app\n")
        .write("pyproject.toml", "[project]\nname = 'app'\n");
    let base = r.commit("base");
    (r, base)
}

fn run(r: &TestRepo, base: &str, opts: Options) -> Selection {
    let project = Project::load(r.path()).unwrap();
    let graph = Graph::build(&project);
    let changes = changed_files(r.path(), &format!("{base}..HEAD")).unwrap();
    select(&project, &graph, &changes, &opts)
}

fn tests_of(sel: &Selection) -> Vec<String> {
    sel.selected_tests
        .iter()
        .map(|p| p.display().to_string())
        .collect()
}

#[test]
fn leaf_change_selects_only_its_tests() {
    let (r, base) = base_repo();
    r.write(
        "app/utils.py",
        "def slug(s):\n    return s.strip().lower()\n",
    );
    r.commit("tweak slug");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Selected);
    assert_eq!(tests_of(&sel), vec!["tests/test_utils.py"]);
    assert_eq!(sel.total_tests, 3);
}

#[test]
fn transitive_change_selects_dependents() {
    let (r, base) = base_repo();
    r.write("app/models.py", "class User:\n    name = ''\n");
    r.commit("add name");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Selected);
    assert_eq!(
        tests_of(&sel),
        vec!["tests/test_models.py", "tests/test_views.py"]
    );
    assert_eq!(
        sel.affected_modules,
        vec!["app.models", "app.views", "test_models", "test_views"]
    );
}

#[test]
fn docs_only_change_selects_nothing() {
    let (r, base) = base_repo();
    r.write("README.md", "# app\n\nnow with docs\n");
    r.commit("docs");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Selected);
    assert!(sel.selected_tests.is_empty());
}

#[test]
fn changed_test_selects_itself() {
    let (r, base) = base_repo();
    r.write(
        "tests/test_utils.py",
        "from app.utils import slug\n\ndef test_x():\n    pass\n",
    );
    r.commit("more tests");

    let sel = run(&r, &base, Options::default());
    assert_eq!(tests_of(&sel), vec!["tests/test_utils.py"]);
}

#[test]
fn conftest_change_runs_everything() {
    let (r, base) = base_repo();
    r.write(
        "tests/conftest.py",
        "import pytest\n\n@pytest.fixture\ndef x():\n    return 1\n",
    );
    r.commit("fixture");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Full);
    assert_eq!(sel.selected_tests.len(), 3);
    assert!(sel.reasons[0].contains("conftest.py"));
}

#[test]
fn config_change_runs_everything() {
    let (r, base) = base_repo();
    r.write("pyproject.toml", "[project]\nname = 'app'\nversion = '2'\n");
    r.commit("bump");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Full);
    assert!(sel.reasons[0].contains("pyproject.toml"));
}

#[test]
fn unknown_data_file_runs_everything() {
    let (r, base) = base_repo();
    r.write("app/templates/index.html", "<h1>hi</h1>\n");
    r.commit("template");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.mode, Mode::Full);
}

#[test]
fn conftest_imports_count_for_every_test_below_it() {
    let (r, _) = base_repo();
    r.write(
        "tests/conftest.py",
        "import pytest\nfrom app.utils import slug\n",
    );
    let base2 = r.commit("conftest uses utils");
    r.write("app/utils.py", "def slug(s):\n    return s\n");
    r.commit("change utils");

    let sel = run(&r, &base2, Options::default());
    assert_eq!(sel.mode, Mode::Full);
    assert_eq!(sel.reasons, vec!["every test can reach the change"]);
}

#[test]
fn dynamic_import_pulls_in_tests_that_reach_it() {
    let (r, _) = base_repo();
    r.write(
        "app/views.py",
        "import importlib\nfrom .models import User\n\ndef load(name):\n    return importlib.import_module(name)\n",
    );
    let base2 = r.commit("plugin loader");
    r.write("app/utils.py", "def slug(s):\n    return s\n");
    r.commit("change utils");

    let sel = run(&r, &base2, Options::default());
    assert_eq!(sel.mode, Mode::Selected);
    assert_eq!(
        tests_of(&sel),
        vec!["tests/test_utils.py", "tests/test_views.py"]
    );
    assert!(sel.reasons.iter().any(|r| r.contains("app/views.py:5")));

    // opting out only follows real imports
    let sel = run(
        &r,
        &base2,
        Options {
            trust_dynamic: true,
        },
    );
    assert_eq!(tests_of(&sel), vec!["tests/test_utils.py"]);
}

#[test]
fn deleted_module_marks_its_package() {
    let (r, base) = base_repo();
    r.remove("app/utils.py")
        .write("tests/test_utils.py", "def test_nothing():\n    pass\n");
    r.commit("drop utils");

    let sel = run(&r, &base, Options::default());
    // anything doing `from app import utils` would go through app/__init__.py
    assert!(sel.affected_modules.contains(&"app".to_string()));
    assert_eq!(sel.mode, Mode::Full);
}

#[test]
fn string_refs_link_tests_to_modules() {
    let (r, _) = base_repo();
    r.write(
        "tests/test_plugins.py",
        "def test_load(load):\n    load('app.utils:slug')\n",
    );
    let base2 = r.commit("plugin test");
    r.write("app/utils.py", "def slug(s):\n    return s\n");
    r.commit("change utils");

    let sel = run(&r, &base2, Options::default());
    assert_eq!(
        sel.selected_tests,
        vec![
            PathBuf::from("tests/test_plugins.py"),
            PathBuf::from("tests/test_utils.py")
        ]
    );
}

#[test]
fn respects_pytest_python_files() {
    let r = TestRepo::new();
    r.write(
        "pyproject.toml",
        "[tool.pytest.ini_options]\npython_files = [\"tests.py\", \"test_*.py\"]\n",
    )
    .write("shop/__init__.py", "")
    .write("shop/models.py", "x = 1\n")
    .write("shop/tests.py", "from shop.models import x\n")
    .write("other/__init__.py", "")
    .write("other/tests.py", "import os\n");
    let base = r.commit("base");
    r.write("shop/models.py", "x = 2\n");
    r.commit("change");

    let sel = run(&r, &base, Options::default());
    assert_eq!(sel.total_tests, 2);
    assert_eq!(tests_of(&sel), vec!["shop/tests.py"]);
}
