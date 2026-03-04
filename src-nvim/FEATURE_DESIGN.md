# Kenjutu Neovim Plugin — Feature Design

## Goal

Per-commit code review inside Neovim. Browse the jj commit graph, select a
commit, review its changes file-by-file, and track review progress — all
without leaving the editor.

## Window Management

All kenjutu views live in a dedicated tab. The user's existing tabs and window
layouts are never modified.

- `:Kenjutu log` opens a new tab with the commit graph
- Entering review replaces the tab content with the review layout
- `q` from review returns to the commit log (same tab)
- `q` from the commit log closes the tab, returning to the user's previous tab

The user can switch between the kenjutu tab and their working tabs freely with
`gt`/`gT` to reference source code while reviewing.

## Screens

### Commit Log

A full-screen buffer showing the jj commit graph with color. The graph is
rendered by jj itself so it looks identical to `jj log` in the terminal.

```
@  sqvmxnoy  yuto  2m ago
│  refactor: move Args to command modules
○  kpwtlqrz  yuto  15m ago
│  feat: add file list command
│ ○  tvmnrqss  yuto  1h ago
├─╯  fix: handle empty diff
◆  zzzzzzzz  root()
```

- `j`/`k` — move between commits (skips graph-only lines)
- `Enter` — open the review screen for the selected commit
- `r` — refresh
- `q` — close

### Review Screen

Opens when pressing `Enter` on a commit. Two panels side by side:

```
┌─ Files 1/3 ──────────┬──────────────────────────────────────────────┐
│                       │                                              │
│  [x]  src/auth.rs     │  (native Neovim diff view)                   │
│  [~]  src/core.rs     │                                              │
│  [ ]  README.md       │   fn connect() {                             │
│                       │ +     validate();                            │
│                       │ +     let conn = open();                     │
│                       │       send(conn);                            │
│                       │   }                                          │
│                       │                                              │
└───────────────────────┴──────────────────────────────────────────────┘
```

#### File List (left panel)

Shows changed files with review status.

- `[x]` reviewed
- `[~]` partially reviewed
- `[ ]` unreviewed
- `[!]` reviewed but file content changed since

Keymaps:

- `j`/`k` — navigate files (immediately loads diff in the right panel)
- `Space` — toggle entire file as reviewed / unreviewed
- `q` — close review, return to commit log

#### Diff View (right panel)

Uses Neovim's native `:diffthis`. The layout adapts based on review state:

**Unreviewed file** — single diff: marker (= base) vs target.

```
┌─ Remaining (Marker → Target) ───────────────────────────┐
│   fn connect() {                                         │
│ +     validate();                                        │
│ +     let conn = open();                                 │
│       send(conn);                                        │
│   }                                                      │
└──────────────────────────────────────────────────────────┘
```

**Partially reviewed file** — two diffs stacked or side-by-side, showing both
what remains to review and what has already been reviewed:

```
┌─ Remaining (Marker → Target) ─┬─ Reviewed (Base → Marker) ──┐
│   fn connect() {               │   fn connect() {             │
│ +     let conn = open();       │ +     validate();            │
│       send(conn);              │       send(conn);            │
│   }                            │   }                          │
└────────────────────────────────┴──────────────────────────────┘
```

The "Remaining" pane shows what still needs review (diff from marker to target).
The "Reviewed" pane shows what has been marked reviewed (diff from base to
marker). Marking hunks moves them from the left pane to the right.

**Fully reviewed file** — the remaining pane is empty, only the reviewed pane
is shown.

Keymaps:

- `Space` (normal) — mark the hunk under cursor as reviewed (in remaining
  pane) or unmark it (in reviewed pane)
- `Space` (visual) — mark/unmark the selected region
- `]c` / `[c` — next / previous change (built-in Neovim diff motions)
- `n` / `N` — next / previous file
- `Tab` — move focus back to file list
- `q` — close diff, return to file list

## Workflow

1. `:Kenjutu log` opens the commit graph
2. Navigate to a commit and press `Enter`
3. The file list loads showing all changed files
4. Select a file — the diff appears on the right
5. Review the diff, press `Space` on hunks or visual-select regions to mark
   them reviewed
6. File status updates automatically (`[ ]` → `[~]` → `[x]`)
7. Move to the next file with `n` or navigate the file list
8. Press `q` to return to the commit log and review another commit

## Alternate Entry Point

`:Kenjutu review <commit>` opens the review screen directly for a given commit,
skipping the commit log.
