#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use git2::{IndexAddOption, Repository, Signature};
use tempfile::TempDir;

/// Throwaway git repo for tests.
pub struct TestRepo {
    pub dir: TempDir,
    pub repo: Repository,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        TestRepo { dir, repo }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write(&self, rel: &str, contents: &str) -> &Self {
        let p: PathBuf = self.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
        self
    }

    pub fn remove(&self, rel: &str) -> &Self {
        fs::remove_file(self.path().join(rel)).unwrap();
        self
    }

    pub fn rename(&self, from: &str, to: &str) -> &Self {
        let to_path = self.path().join(to);
        fs::create_dir_all(to_path.parent().unwrap()).unwrap();
        fs::rename(self.path().join(from), to_path).unwrap();
        self
    }

    /// Stage everything and commit. Returns the commit sha.
    pub fn commit(&self, msg: &str) -> String {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.update_all(["*"].iter(), None).unwrap();
        index.write().unwrap();
        let tree = self.repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("test", "test@example.com").unwrap();
        let parents: Vec<_> = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parent_refs: Vec<_> = parents.iter().collect();
        self.repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .unwrap()
            .to_string()
    }
}
