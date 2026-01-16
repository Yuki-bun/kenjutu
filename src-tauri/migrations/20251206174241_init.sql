-- Add migration script here
DROP TABLE IF EXISTS repository;

CREATE TABLE IF NOT EXISTS repository
(
    gh_id  TEXT NOT NULL,
    local_dir       TEXT,
    owner           TEXT NOT NULL,
    name            TEXT NOT NULL,

    PRIMARY KEY (gh_id)
);

-- Create unique index for reverse lookup (owner, name) -> gh_repo_id
CREATE UNIQUE INDEX IF NOT EXISTS idx_repository_owner_name
    ON repository(owner, name);
 
CREATE TABLE IF NOT EXISTS reviewed_files (
    gh_repo_id TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    change_id TEXT,  -- NULL for non-jujutsu commits
    file_path TEXT NOT NULL,
    patch_id TEXT NOT NULL,
    reviewed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (gh_repo_id, pr_number, change_id, file_path, patch_id),
    FOREIGN KEY (gh_repo_id) REFERENCES repository(gh_id)
);

-- Index for efficient queries by PR
CREATE INDEX idx_reviewed_files_pr
    ON reviewed_files(gh_repo_id, pr_number);
