-- Create table for tracking reviewed files
-- Tracks files by (change-id, file-path, patch-id) for rebase resilience
CREATE TABLE IF NOT EXISTS reviewed_files (
    github_node_id TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    change_id TEXT,  -- NULL for non-jujutsu commits
    file_path TEXT NOT NULL,
    patch_id TEXT NOT NULL,
    reviewed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (github_node_id, pr_number, change_id, file_path, patch_id),
    FOREIGN KEY (github_node_id) REFERENCES repository(github_node_id)
);

-- Index for efficient queries by PR
CREATE INDEX idx_reviewed_files_pr
    ON reviewed_files(github_node_id, pr_number);
