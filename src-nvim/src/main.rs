mod commands;

use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kjn", about = "Kenjutu CLI for Neovim integration")]
struct Cli {
    /// Path to the repository directory
    #[arg(short, long, default_value = ".", global = true)]
    dir: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Output the jj commit graph as JSON
    Log,

    /// List changed files with review status for a commit
    Files(commands::files::FilesArgs),

    /// Output file content from base, marker, or target tree (raw, not JSON)
    Blob(commands::blob::BlobArgs),

    /// Mark a file as reviewed
    MarkFile(commands::mark::MarkFileArgs),

    /// Unmark a file as reviewed
    UnmarkFile(commands::mark::UnmarkFileArgs),

    /// Mark a hunk as reviewed
    MarkHunk(commands::mark_hunk::MarkHunkArgs),

    /// Unmark a hunk as reviewed
    UnmarkHunk(commands::mark_hunk::UnmarkHunkArgs),
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        let err = serde_json::json!({ "error": format!("{e:#}") });
        eprintln!("{}", serde_json::to_string(&err).unwrap());
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let local_dir = std::fs::canonicalize(&cli.dir).context("invalid directory")?;

    match cli.command {
        Command::Log => commands::log::run(&local_dir),
        Command::Files(args) => commands::files::run(&local_dir, args),
        Command::Blob(args) => commands::blob::run(&local_dir, args),
        Command::MarkFile(args) => commands::mark::run_mark(&local_dir, args),
        Command::UnmarkFile(args) => commands::mark::run_unmark(&local_dir, args),
        Command::MarkHunk(args) => commands::mark_hunk::run_mark(&local_dir, args),
        Command::UnmarkHunk(args) => commands::mark_hunk::run_unmark(&local_dir, args),
    }
}
