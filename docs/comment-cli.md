# Kenjutu Comment CLI

A command-line tool (`kjc`) for retrieving inline diff comments attached to jj changes.
Designed primarily for consumption by AI agents — it outputs structured JSON with 
file paths, line numbers, comment bodies, and surrounding source context so that
agents can understand and act on review feedback.

## Features

- **Agent-friendly JSON** — Structured output with file paths, line numbers, context, and threading that agents can parse directly
- **Auto-detection** — Automatically detects the change ID from your jj working copy
- **File filtering** — Narrow results to a specific file
- **Resolved/unresolved** — Shows only unresolved comments by default; use `--all` to include resolved ones
- **Context lines** — Each comment includes surrounding source lines so agents can locate and understand the code being discussed

## Prerequisites

- [Rust toolchain](https://rustup.rs/)
- [Jujutsu](https://martinvonz.github.io/jj/) (`jj` CLI, v0.38+)

## Installation

```bash
# From the kenjutu repository
cargo build --release -p kenjutu-comments

# The binary will be at target/release/kjc
```

## Usage

```bash
# Show unresolved comments for the current working copy change
kjc

# Specify a repository directory
kjc --dir /path/to/repo

# Specify a change ID explicitly
kjc --change-id ksrmyxvnwqpqrqxpvrts

# Filter to a specific file
kjc --file src/main.rs

# Include resolved comments too
kjc --all
```

### Flags

| Flag               | Short | Description                                          |
| ------------------ | ----- | ---------------------------------------------------- |
| `--dir <path>`     | `-d`  | Path to the repository (default: `.`)                |
| `--change-id <id>` | `-c`  | Full-length jj change ID (auto-detected if omitted)  |
| `--file <path>`    | `-f`  | Filter to a specific file                            |
| `--all`            | `-a`  | Include resolved comments (default: unresolved only) |

## Output Format

```json
{
  "files": [
    {
      "path": "src/main.rs",
      "comments": [
        {
          "line": 42,
          "side": "new",
          "body": "Consider extracting this into a helper function.",
          "target_sha": "abc123...",
          "resolved": false,
          "context": {
            "before": "fn main() {\n    let config = load_config();",
            "target": "    let result = complex_operation(config, args, flags);",
            "after": "    println!(\"{result}\");\n}"
          },
          "replies": ["Good point, I'll refactor this."]
        }
      ]
    }
  ]
}
```

### Fields

- **`line`** — The ported line number in the current version of the file
- **`start_line`** — Start line for multi-line comments (omitted for single-line)
- **`side`** — Which side of the diff: `"old"` (deletion) or `"new"` (addition)
- **`body`** — The comment text
- **`target_sha`** — The commit SHA the comment was originally placed on
- **`resolved`** — Whether the comment has been marked as resolved
- **`context`** — Surrounding source lines (up to 3 before, the target line(s), up to 3 after)
- **`replies`** — List of reply bodies in chronological order
