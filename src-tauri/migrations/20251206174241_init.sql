-- Add migration script here
DROP TABLE IF EXISTS repository;

CREATE TABLE IF NOT EXISTS repository
(
    github_node_id  TEXT NOT NULL,
    local_dir       TEXT NOT NULL,

    PRIMARY KEY (github_node_id)
);
