use std::env;

use git2::{AutotagOption, Commit, Cred, FetchOptions, Oid, RemoteCallbacks, Repository};

use crate::models::ChangeId;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
}

pub fn open_repository(local_dir: &str) -> Result<Repository> {
    Repository::open(local_dir).map_err(|_| Error::RepoNotFound(local_dir.to_string()))
}

pub fn get_or_fetch_commit(repo: &Repository, oid: Oid) -> Result<Commit<'_>> {
    if let Ok(commit) = repo.find_commit(oid) {
        return Ok(commit);
    }

    let mut remote = repo.find_remote("origin")?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        // TODO: Support configurable SSH key paths
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);
    // Ensure we don't accidentally pull in tags we don't want
    fo.download_tags(AutotagOption::None);

    // Don't create a ref
    let refspec = format!("{}:", oid);

    remote.fetch(&[&refspec], Some(&mut fo), None)?;

    repo.find_commit(oid)
        .map_err(|_| Error::CommitNotFound(oid.to_string()))
}

#[must_use = "The underling ref object will be deleted when this struct is dropped"]
pub struct TemporaryRef<'a> {
    reference: git2::Reference<'a>,
}

impl<'a> Drop for TemporaryRef<'a> {
    fn drop(&mut self) {
        log::info!(
            "Deleting temporary reference {}",
            self.reference.name().unwrap_or("")
        );
        if let Err(err) = self.reference.delete() {
            log::error!(
                "Failed to delete temporary reference {}: {}",
                self.reference.name().unwrap_or(""),
                err
            );
        }
    }
}

/// Store commits under refs/remotes/revue so that jj can find the commit
pub fn store_commit_as_fake_remote<'a>(
    repo: &'a Repository,
    commit: &'a Commit<'a>,
) -> Result<TemporaryRef<'a>> {
    let oid = commit.id();
    let ref_name = format!("refs/remotes/revue/{}", oid);
    let reference = repo.reference(
        &ref_name,
        oid,
        true,
        &format!("Pinning  commit for PR {}", oid),
    )?;

    Ok(TemporaryRef { reference })
}

pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
    commit
        .header_field_bytes("change-id")
        .ok()
        .and_then(|buf| buf.as_str().map(String::from).map(ChangeId::from))
}
