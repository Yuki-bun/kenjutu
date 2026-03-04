use anyhow::{Context, Result};
use kenjutu_core::services::graph;
use std::path::Path;

pub fn run(local_dir: &Path) -> Result<()> {
    let commit_graph = graph::get_log_graph(local_dir).context("failed to get jj log graph")?;

    println!("{}", serde_json::to_string(&commit_graph)?);
    Ok(())
}
