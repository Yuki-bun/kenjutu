-- Add migration script here
DROP TABLE IF EXISTS repository;

CREATE TABLE IF NOT EXISTS repository
(
    github_id   INTEGER NOT NULL,
    local_dir   TEXT NOT NULL,

    PRIMARY KEY (github_id, local_dir)
);
