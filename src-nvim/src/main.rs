mod serve;

use std::{path::PathBuf, process};

use anyhow::{Context, Result, anyhow, bail};

fn main() -> Result<()> {
    let args = parse_args()?;
    match args {
        Args::Server { dir } => {
            if let Err(e) = serve::run(&dir) {
                let err = serde_json::json!({ "error": format!("{e:#}") });
                eprintln!("{}", serde_json::to_string(&err).unwrap());
                process::exit(1);
            }
        }
        Args::Version => {
            let version = env!("CARGO_PKG_VERSION");
            println!("v{}", version);
        }
    }

    Ok(())
}

enum Args {
    Server { dir: PathBuf },
    Version,
}

fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let Some(first_arg) = args.next() else {
        return Ok(Args::Server {
            dir: PathBuf::from("."),
        });
    };

    match first_arg.as_str() {
        "--version" => Ok(Args::Version),
        "--dir" | "-d" => {
            let dir = args
                .next()
                .ok_or(anyhow!("--dir requires a value"))
                .and_then(|dir| std::fs::canonicalize(&dir).context("invalid directory"))?;
            Ok(Args::Server { dir })
        }
        _ => bail!("unknown argument {}", first_arg),
    }
}
