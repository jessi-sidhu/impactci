mod common;

use std::path::PathBuf;

use blast::diff::{ChangeKind, changed_files};
use common::TestRepo;

fn summary(repo: &TestRepo, range: &str) -> Vec<(String, ChangeKind)> {
    changed_files(repo.path(), range)
        .unwrap()
        .into_iter()
        .map(|c| (c.path.display().to_string(), c.kind))
        .collect()
}

#[test]
fn added_modified_deleted() {
    let r = TestRepo::new();
    r.write("pkg/__init__.py", "")
        .write("pkg/a.py", "x = 1\n")
        .write("pkg/b.py", "y = 2\n");
    let base = r.commit("base");

    r.write("pkg/a.py", "x = 2\n")
        .remove("pkg/b.py")
        .write("pkg/c.py", "z = 3\n");
    r.commit("change");

    assert_eq!(
        summary(&r, &format!("{base}..HEAD")),
        vec![
            ("pkg/a.py".into(), ChangeKind::Modified),
            ("pkg/b.py".into(), ChangeKind::Deleted),
            ("pkg/c.py".into(), ChangeKind::Added),
        ]
    );
    // bare base means base..HEAD
    assert_eq!(summary(&r, &base).len(), 3);
}

#[test]
fn renames_keep_old_path() {
    let r = TestRepo::new();
    r.write(
        "old.py",
        "def f():\n    return 'some reasonably long body'\n",
    );
    let base = r.commit("base");
    r.rename("old.py", "new.py");
    r.commit("rename");

    let changes = changed_files(r.path(), &format!("{base}..")).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Renamed);
    assert_eq!(changes[0].path, PathBuf::from("new.py"));
    assert_eq!(changes[0].old_path, Some(PathBuf::from("old.py")));
}

#[test]
fn diffs_from_merge_base() {
    let r = TestRepo::new();
    r.write("a.py", "1\n");
    let root = r.commit("root");

    // "main" moves on after the branch point
    r.write("main_only.py", "1\n");
    let main = r.commit("main work");

    // branch off root
    let root_commit = r
        .repo
        .find_commit(git2::Oid::from_str(&root).unwrap())
        .unwrap();
    r.repo.branch("feature", &root_commit, false).unwrap();
    r.repo.set_head("refs/heads/feature").unwrap();
    r.repo
        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    r.write("feature.py", "1\n");
    r.commit("feature work");

    assert_eq!(
        summary(&r, &format!("{main}..HEAD")),
        vec![("feature.py".into(), ChangeKind::Added)]
    );
}

#[test]
fn bad_revision_errors() {
    let r = TestRepo::new();
    r.write("a.py", "");
    r.commit("root");
    assert!(changed_files(r.path(), "nope..HEAD").is_err());
}
