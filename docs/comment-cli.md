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
- **Comment status** — Quick overview of unresolved/resolved comment counts per change

## Prerequisites

- [Jujutsu](https://martinvonz.github.io/jj/) (`jj` CLI, v0.38+)

## Installation

Download a binary from the [releases page](https://github.com/Yuki-bun/kenjutu/releases) (look for `kjc/v*` tags) and place it on your `PATH`:

```bash
VERSION=0.1.0

# macOS (Apple Silicon)
curl -LO "https://github.com/Yuki-bun/kenjutu/releases/download/kjc/v${VERSION}/kjc-aarch64-darwin"
chmod +x kjc-aarch64-darwin
sudo mv kjc-aarch64-darwin /usr/local/bin/kjc

# Linux (x86_64)
curl -LO "https://github.com/Yuki-bun/kenjutu/releases/download/kjc/v${VERSION}/kjc-x86_64-linux"
chmod +x kjc-x86_64-linux
sudo mv kjc-x86_64-linux /usr/local/bin/kjc

# Linux (aarch64)
curl -LO "https://github.com/Yuki-bun/kenjutu/releases/download/kjc/v${VERSION}/kjc-aarch64-linux"
chmod +x kjc-aarch64-linux
sudo mv kjc-aarch64-linux /usr/local/bin/kjc
```

### From source

```bash
git clone --depth 1 https://github.com/Yuki-bun/kenjutu.git
cd kenjutu
cargo install --path src-cli
```

## Usage

`kjc` has two subcommands: `get` and `status`.

### `kjc get` — Retrieve comments

```bash
# Show unresolved comments for the current working copy change
kjc get

# Specify a change ID explicitly
kjc get -c ksrmyxvnwqpqrqxpvrts

# Filter to a specific file
kjc get -f src/main.rs

# Include resolved comments too
kjc get -a
```

#### Flags

| Flag               | Short | Description                                          |
| ------------------ | ----- | ---------------------------------------------------- |
| `--change-id <id>` | `-c`  | Full-length jj change ID (auto-detected if omitted)  |
| `--file <path>`    | `-f`  | Filter to a specific file                            |
| `--all`            | `-a`  | Include resolved comments (default: unresolved only) |

### `kjc status` — Show comment counts

```bash
# Comment counts for the default revset
kjc status

# Comment counts for a specific revset
kjc status "trunk()..@"

# Include changes with no unresolved comments
kjc status -a
```

#### Flags

| Flag    | Short | Description                                 |
| ------- | ----- | ------------------------------------------- |
| `--all` | `-a`  | Include changes with no unresolved comments |

### Global flags

| Flag           | Short | Description                           |
| -------------- | ----- | ------------------------------------- |
| `--dir <path>` | `-d`  | Path to the repository (default: `.`) |

## Output Format

### `kjc get`

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

### `kjc status`

```json
[
  {
    "change_id": "ksrmyxvnwqpqrqxpvrts",
    "description": "feat: add user authentication",
    "unresolved": 3,
    "resolved": 1
  }
]
```

### Fields (`get`)

- **`line`** — The ported line number in the current version of the file
- **`start_line`** — Start line for multi-line comments (omitted for single-line)
- **`side`** — Which side of the diff: `"old"` (deletion) or `"new"` (addition)
- **`body`** — The comment text
- **`target_sha`** — The commit SHA the comment was originally placed on
- **`resolved`** — Whether the comment has been marked as resolved
- **`context`** — Surrounding source lines (up to 3 before, the target line(s), up to 3 after)
- **`replies`** — List of reply bodies in chronological order

### Fields (`status`)

- **`change_id`** — The full jj change ID
- **`description`** — First line of the commit description
- **`unresolved`** — Number of unresolved comments
- **`resolved`** — Number of resolved comments
