use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
};

use git2::Repository;

use crate::{ChangeId, Result};

#[derive(Debug)]
pub struct MarkerCommitLock {
    path: PathBuf,
    change_id: ChangeId,
    _lock_file: File,
}

impl MarkerCommitLock {
    pub fn new(repo: &Repository, change_id: ChangeId) -> Result<Self> {
        let path = Self::lock_path(repo, &change_id);
        fs::create_dir_all(path.parent().unwrap())?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        file.lock_exclusive()?;

        log::info!("created lock file at {}", path.to_str().unwrap_or(""));
        Ok(Self {
            _lock_file: file,
            change_id,
            path,
        })
    }

    pub fn lock_path(repo: &Repository, change_id: &ChangeId) -> PathBuf {
        repo.path()
            .join("info/kenjutu/lock/")
            .join(change_id.as_ref())
    }
}

impl Drop for MarkerCommitLock {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.path) {
            log::warn!(
                "failed to delete lock file for change_id {}. error: {}",
                self.change_id.as_ref(),
                err
            );
        }
        log::info!("deleted lock file at {}", self.path.to_str().unwrap_or(""))
    }
}

#[cfg(test)]
mod tests {
    use test_repo::TestRepo;

    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_marker_mutual_exclusion() -> Result<(), Box<dyn std::error::Error>> {
        let repo = TestRepo::new()?;
        let dir = repo.path();
        let change_id = ChangeId::from("test-change-id".to_string());
        let active_threads = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..20 {
            let active_threads = Arc::clone(&active_threads);
            let repo = Repository::open(dir)?;
            let change_id = change_id.clone();
            handles.push(thread::spawn(move || {
                let lock = MarkerCommitLock::new(&repo, change_id.clone()).unwrap();
                let current = active_threads.fetch_add(1, Ordering::SeqCst);
                assert!(
                    current == 0,
                    "concurrent access to marker commit is not allowed"
                );
                thread::sleep(Duration::from_millis(50));
                active_threads.fetch_sub(1, Ordering::SeqCst);
                drop(lock);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        Ok(())
    }
}
