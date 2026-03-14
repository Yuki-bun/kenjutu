# Kenjutu

**A per-commit code review system for [Jujutsu](https://martinvonz.github.io/jj/) repositories.**

<p>
<video src="https://github.com/user-attachments/assets/237a3d93-0f1e-4d28-9c98-b2067e73cd26" autoplay loop muted playsinline width="49%"></video>
<video src="https://github.com/user-attachments/assets/e6c8ae3e-aad4-48f0-ba23-6d0be6c381a8" autoplay loop muted playsinline width="49%"></video>
</p>

Kenjutu is a local code review tool for [Jujutsu](https://martinvonz.github.io/jj/)
repositories that use Git as a backend. It lets you review changes commit-by-commit
with hunk-level granularity.

Think of it as having a staging area for every commit — you selectively mark
regions as reviewed, building up your progress hunk by hunk. Review state is
persisted as git objects in your local repository — no database, no external
service. Because review progress is tracked at the content level, it survives
rebases and history rewrites.

> **This is very much a work in progress.** Things will break, features are incomplete,
> and the interface may change significantly. Feedback is welcome!

## Why commit-based development?

When each commit is a self-contained, coherent change, it's easier to reason about
what it does. Clean commits help you organize your own thinking, make pull requests
easier to review commit-by-commit, and leave a git history that explains _why_ code
exists — not just the messy path it took to get there.

This matters even more as we spend more time reviewing AI-generated code — making
each commit self-contained lightens the mental load of verifying what the AI
produced.

Jujutsu makes this workflow practical by treating history as mutable — amending any
commit is as easy as editing the latest one. Kenjutu completes the loop by tracking
your review progress through those rewrites, so you never lose sight of what you've
verified.

## How it compares

|                               | Kenjutu                                             | GitHub / GitLab               | Gerrit                                          |
| ----------------------------- | --------------------------------------------------- | ----------------------------- | ----------------------------------------------- |
| **Review unit**               | Per-commit                                          | PR-level centric              | Per-patchset                                    |
| **Review granularity**        | Hunk-level — partially review a file, pick up later | File-level "Viewed" checkbox  | File-level                                      |
| **Rebase handling**           | Progress persists — tied to jj change IDs           | Progress resets on force-push | Inter-diff between patchsets                    |
| **Comments on local commits** | Yes — before pushing, with `kjc` for coding agents  | Only on pushed PRs            | Only on pushed patchsets                        |
| **Collaboration**             | Local + limited GitHub PR support (desktop)         | Full team workflow            | Full team workflow with code owners             |
| **Hosting**                   | Local — no server needed                            | Cloud / self-hosted           | Self-hosted                                     |

### Remaining diff vs inter-diff

Many review tools use **inter-diff** to handle rebases: they snapshot each push as a
numbered revision and let reviewers diff between revisions. This works well when the
reviewer has internalized the previous version and wants to check for specific changes
that address their feedback.

Kenjutu takes a different approach: **remaining diff**. Instead of tracking revisions,
it tracks which hunks you've verified in the _current_ content. After a rebase or amend,
you see exactly what still needs review — no more, no less.

These solve different problems. Inter-diff answers "what changed since I last looked?"
Remaining diff answers "what haven't I verified yet?" — useful when you're building up
confidence that a commit is correct, which is the core of Kenjutu's review workflow.
Inter-diff can't express partial review progress, and remaining diff doesn't
assume you've already seen a prior version.

## Interfaces

Kenjutu is available in two interfaces, both sharing the same core engine:

| Interface   | Binary | Description                            | Docs                                       |
| ----------- | ------ | -------------------------------------- | ------------------------------------------ |
| **Desktop** | —      | Tauri 2 app with GitHub PR integration | [docs/desktop.md](docs/desktop.md)         |
| **Neovim**  | `kjn`  | Neovim plugin for in-editor review     | [docs/nvim-plugin.md](docs/nvim-plugin.md) |

### Comment CLI

Kenjutu also ships `kjc`, a utility that outputs diff comments as structured
JSON for AI agents. See [docs/comment-cli.md](docs/comment-cli.md).

## Key Features

- **Per-commit review** — Review changes one commit at a time, the way they were authored
- **Hunk-level tracking** — Mark individual hunks as reviewed, not just whole files
- **Built for jj** — Designed around jj's change IDs, and mutable history (requires git backend)
- **Survives history rewrites** — Review state is tied to jj's change IDs, not commit hashes. Amend, rebase, or squash freely — your review progress stays with it.
- **Review state as git objects** — Review progress is stored as git objects in your repo, no database or external service
- **GitHub PR support** — View and review pull requests locally (desktop app)
- **Inline comments** — Comment on any local commit before pushing, with threaded replies and resolve/unresolve. `kjc` outputs comments as structured JSON designed for AI agent consumption.

## Tech Stack

- **Core**: Rust — git2 for git ops, jj CLI for commit graph and status
- **Desktop**: React 19 + Tauri 2
- **Neovim**: Lua plugin + Rust CLI backend (`kjn`)

## Getting Started

Each interface has its own installation guide — pick the one that fits your workflow:

- [Desktop App](docs/desktop.md) — Full-featured GUI with GitHub integration
- [Neovim Plugin](docs/nvim-plugin.md) — Stay in your editor

For AI-facing comment retrieval, see [Comment CLI](docs/comment-cli.md).

All interfaces require [Jujutsu](https://martinvonz.github.io/jj/) (v0.38+) to be installed.

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
