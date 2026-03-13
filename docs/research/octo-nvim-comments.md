# Octo.nvim Comment UI/UX Research

Research on how [octo.nvim](https://github.com/pwntester/octo.nvim) implements
PR review comments in its diff view.

## Diff View Layout

- **2-pane vertical split** in a dedicated tab page: left = original, right = modified
- Both panes are scroll-bound and cursor-bound (`diffthis`)
- A **file panel** at the bottom lists changed files with diffstat, view state, and comment counts
- TreeSitter syntax highlighting; hunks folded by default

```
┌─────────────────────┬─────────────────────┐
│                     │                     │
│   Left (original)   │  Right (modified)   │
│                     │                     │
│                     │                     │
├─────────────────────┴─────────────────────┤
│  File panel: changed files + stats        │
└───────────────────────────────────────────┘
```

When a thread is shown, the alt pane swaps to the thread buffer:

```
┌─────────────────────┬─────────────────────┐
│                     │  Thread panel:       │
│   Left (original)   │   code snippet       │
│   or Right diff     │   comment 1          │
│                     │   comment 2          │
│                     │   [reply area]       │
├─────────────────────┴─────────────────────┤
│  File panel: changed files + stats        │
└───────────────────────────────────────────┘
```

## Comment Indicators in the Diff

### Sign column

Colored `▎` bars in the sign column mark lines with comments:

| Sign | Color | Meaning |
|------|-------|---------|
| `octo_thread` | Blue | Active thread |
| `octo_thread_resolved` | Green | Resolved thread |
| `octo_thread_outdated` | Red | Outdated thread |
| `octo_thread_pending` | Yellow | Pending (not yet submitted) |

### Valid comment ranges

Lines within diff hunks get a separate sign (`octo_comment_range`) with green
line-number highlighting, telling users "you can comment here." Ranges are
extracted from the patch with `utils.process_patch()`.

### Virtual text

On the first line of a commented range, right-aligned virtual text shows count + date:
```
    2 comments (3 hours ago)
```

## Thread Display

When the cursor rests on a commented line (`CursorHold` event), the
**alternative pane** swaps its buffer to show the thread panel. This is
automatic (`auto_show_threads = true` by default).

Thread buffer URI format: `octo://{repo}/review/{id}/threads/{side}/{path}:{line}`

### Thread buffer contents

1. **Header**: timeline marker + "THREAD:" label, file path, line range,
   commit abbrev, status badges (Outdated in red, Resolved in green)
2. **Code snippet**: the commented lines with syntax highlighting and original
   line numbers as virtual text
3. **Comments**: author, timestamp, body, reactions — each comment is foldable
4. **Reply area**: an editable region at the bottom

### Thread interaction

- `q` hides the thread and restores the diff pane
- `]t` / `[t` navigate between threads
- Comments within a thread are folded by default; the whole thread can be
  collapsed/expanded
- Threads resolve/unresolve with `<localleader>rt` / `<localleader>rT`

## Creating Comments

1. User selects lines in visual mode (or single line in normal mode) in either
   diff pane
2. Presses `<localleader>ca` (comment) or `<localleader>sa` (suggestion)
3. Plugin validates the selection is within a diff hunk — rejects comments
   outside diff hunks
4. A **stub thread** is created locally (`id = -1`, `state = PENDING`)
5. The alt pane swaps to show a thread buffer with an editable comment body
6. User enters insert mode and types their comment
7. Comment is stored locally as pending — **not submitted yet**

### Suggestion comments

For suggestions, the selected code lines are pre-filled into a fenced code
block:

````
```suggestion
original line 1
original line 2
```
````

GitHub recognizes this syntax and renders an "Apply suggestion" button.

## Review Submission Flow

```
Start review (:OctoReview start)
  → Navigate files (]f / [f / ]u for next unviewed)
  → View 2-pane diffs
  → Add comments on selected lines
  → Navigate threads (]t / [t)
  → Resolve/unresolve threads
  → Submit review (:OctoReview submit)
```

On submit:
- A centered floating window opens for the review summary message
- All pending comments across all files are collected and submitted atomically
  via a single GraphQL mutation
- The review tab closes

## Key Data Structures

### ReviewThread
```
originalStartLine, originalLine    -- PR diff lines
line, startLine                    -- current lines (may differ if focused on specific commit)
startDiffSide, diffSide            -- "LEFT" or "RIGHT"
path                               -- file path
isOutdated, isResolved, isCollapsed
id                                 -- string or -1 for pending local threads
comments.nodes[]                   -- array of ReviewComment
```

### CommentMetadata
```
id, author, body, savedBody, dirty
state                              -- "PENDING" or "PUBLISHED"
path, diffSide
snippetStartLine, snippetEndLine
pullRequestReview.id
```

### FileEntry
```
path, patch
left_comment_ranges, right_comment_ranges  -- [start, end][] valid for commenting
left_bufid, right_bufid                    -- diff buffer IDs
associated_bufs                            -- thread buffer IDs for this file
viewed_state                               -- "VIEWED" / "UNVIEWED" / "DISMISSED"
diffhunks                                  -- extracted from patch
```

## Notable Design Decisions

1. **No floating windows for threads** — threads replace one side of the diff
   in a full split, giving ample space for long conversations

2. **Auto-show/hide via CursorHold** — zero-click thread viewing; feels like
   hover tooltips but in a full pane

3. **Pending comments are local-only** until review submission — enables batch
   review workflow matching GitHub's model

4. **Comment ranges derived from patch hunks** — users can only comment on
   lines within the diff, matching GitHub's behavior

5. **Thread buffers are regular Neovim buffers** — editable with `:w` to save,
   supporting all normal editing commands

6. **Reactions supported inline** — toggle emoji reactions
   (`<localleader>r{key}`) without leaving the buffer

## Source Files

| Functionality | File |
|---|---|
| Review initialization & flow | `lua/octo/reviews/init.lua` |
| Thread display in alt pane | `lua/octo/reviews/thread-panel.lua` |
| Layout (2-pane split) | `lua/octo/reviews/layout.lua` |
| File diff buffers & signs | `lua/octo/reviews/file-entry.lua` |
| Thread & comment rendering | `lua/octo/ui/writers.lua` |
| Sign definitions | `lua/octo/ui/signs.lua` |
| Patch processing | `lua/octo/utils.lua` |
| Comment persistence | `lua/octo/model/octo-buffer.lua` |
