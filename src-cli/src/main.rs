mod get;
mod resolve;
mod status;

use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kenjutu-comments",
    about = "Retrieve inline diff comments for a jj change"
)]
struct Cli {
    /// Path to the repository directory
    #[arg(short, long, default_value = ".", global = true)]
    dir: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Retrieve ported comments for a change (JSON output)
    Get {
        /// Jujutsu change ID (auto-detected from working copy if omitted)
        #[arg(short, long)]
        change_id: Option<String>,

        /// Filter to a specific file path
        #[arg(short, long)]
        file: Option<String>,

        /// Include resolved comments (default: unresolved only)
        #[arg(short, long, default_value_t = false)]
        all: bool,
    },

    /// Show comment counts per change (JSON output)
    Status {
        /// Jujutsu revset (omit to use jj's default revset)
        revset: Option<String>,
        /// Include commits with no unresolved comments (default: unresolved only)
        #[arg(short, long, default_value_t = false)]
        all: bool,
    },
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
        Command::Get {
            change_id,
            file,
            all,
        } => get::run(&local_dir, &cli.dir, change_id, file, all),
        Command::Status { revset, all } => status::run(&local_dir, revset, all),
    }
}
