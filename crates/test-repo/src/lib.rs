use std::{ffi::OsStr, path::Path, process::Command};

use git2::{Oid, Repository};
use serde::Deserialize;
use serde_json::Deserializer;
use tempfile::TempDir;

pub struct TestRepo {
    pub repo: Repository,
    _dir: TempDir,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("jj error: {0}")]
    Jj(String),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Deserialize)]
pub struct CommitInfo {
    pub commit_id: String,
    pub change_id: String,
}

impl CommitInfo {
    pub fn oid(&self) -> Oid {
        Oid::from_str(&self.commit_id).expect("Invalid commit ID")
    }
}

pub struct CommitResult {
    /// The commit that is currently @ after the commit command, which is the new commit
    pub work_copy: CommitInfo,
    /// The commit that was created by the commit command, which is the parent of the new commit
    pub created: CommitInfo,
}

impl TestRepo {
    pub fn new() -> Result<Self> {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path())?;
        let test_repo = Self { _dir: dir, repo };
        test_repo.setup_jujutu()?;

        Ok(test_repo)
    }

    pub fn path(&self) -> &str {
        self._dir.path().to_str().unwrap()
    }

    pub fn setup_jujutu(&self) -> Result<()> {
        self.jj().args(["git", "init"]).run()?;

        self.jj()
            .args(["config", "set", "--repo", "user.name", "Test User"])
            .run()?;
        self.jj()
            .args(["config", "set", "--repo", "user.email", "test@test.com"])
            .run()?;

        Ok(())
    }

    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let file_path = self._dir.path().join(path);
        std::fs::create_dir_all(file_path.parent().unwrap())?;
        std::fs::write(&file_path, content)?;
        Ok(())
    }

    pub fn delete_file(&self, path: &str) -> Result<()> {
        let file_path = self._dir.path().join(path);
        std::fs::remove_file(&file_path)?;
        Ok(())
    }

    pub fn rename_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old_file_path = self._dir.path().join(old_path);
        let new_file_path = self._dir.path().join(new_path);
        std::fs::create_dir_all(new_file_path.parent().unwrap())?;
        std::fs::rename(&old_file_path, &new_file_path)?;
        Ok(())
    }

    pub fn merge(&self, parents: &[&str], message: &str) -> Result<CommitInfo> {
        let mut cmd = self.jj().args(["new", "-m", message]);
        for parent in parents {
            cmd = cmd.args([parent]);
        }
        cmd.run()?;
        let output = self
            .jj()
            .args(["log", "-T", "json(self)", "-r", "@", "--no-graph"])
            .run()?;
        let commit = serde_json::from_slice(&output)
            .map_err(|e| Error::Jj(format!("Failed to parse commit info: {}", e)))?;

        Ok(commit)
    }

    pub fn new_revision(&self, revision: &str) -> Result<CommitInfo> {
        self.jj().args(["new", "-r", revision]).run()?;
        self.work_copy()
    }

    pub fn work_copy(&self) -> Result<CommitInfo> {
        let output = self
            .jj()
            .args(["log", "-T", "json(self)", "-r", "@", "--no-graph"])
            .run()?;
        let commit = serde_json::from_slice(&output)
            .map_err(|e| Error::Jj(format!("Failed to parse commit info: {}", e)))?;

        Ok(commit)
    }

    pub fn edit(&self, revision: &str) -> Result<()> {
        self.jj().args(["edit", revision]).run()?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<CommitResult> {
        self.jj().args(["commit", "-m", message]).run()?;

        let log_output = self
            .jj()
            .args(["log", "-T", "json(self)", "-r", "@|@-", "--no-graph"])
            .run()?;

        let stream = Deserializer::from_slice(&log_output).into_iter::<CommitInfo>();
        let commits: Vec<CommitInfo> = stream.map(|c| c.unwrap()).collect();
        assert_eq!(commits.len(), 2, "Expected exactly 2 commits in log output");
        let new_commit = &commits[0];
        let parent_commit = &commits[1];
        Ok(CommitResult {
            work_copy: new_commit.clone(),
            created: parent_commit.clone(),
        })
    }

    fn jj(&self) -> JjCommandBuilder {
        JjCommandBuilder::new(self._dir.path())
    }
}

struct JjCommandBuilder {
    command: Command,
}

impl JjCommandBuilder {
    fn new(dir: &Path) -> Self {
        let mut command = Command::new("jj");
        command.current_dir(dir);
        Self { command }
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    fn run(mut self) -> Result<Vec<u8>> {
        let output = self.command.output().expect("Failed to execute jj command");
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Error::Jj(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}
