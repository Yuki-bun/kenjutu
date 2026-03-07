mod serve;

use std::process;

use anyhow::{Context, Result};

fn main() {
    if let Err(e) = run() {
        let err = serde_json::json!({ "error": format!("{e:#}") });
        eprintln!("{}", serde_json::to_string(&err).unwrap());
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let local_dir = parse_dir()?;
    let local_dir = std::fs::canonicalize(&local_dir).context("invalid directory")?;
    serve::run(&local_dir)
}

fn parse_dir() -> Result<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" | "-d" => {
                return args.next().context("--dir requires a value");
            }
            _ if arg.starts_with("--dir=") => {
                return Ok(arg["--dir=".len()..].to_string());
            }
            _ => {}
        }
    }
    Ok(".".to_string())
}
