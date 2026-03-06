# Neovim Plugin Test Plan

## Framework

**mini.test** — lightweight, zero-dependency test framework that runs inside headless Neovim.

Tests run via:

```bash
make test-lua
# or directly:
nvim --headless --noplugin -u tests/minimal_init.lua -c "lua MiniTest.run()"
```

The `tests/minimal_init.lua` script bootstraps mini.test (downloading it if needed) and
adds the plugin to `runtimepath`. No personal Neovim config is involved.

## Test Structure

```
tests/
  minimal_init.lua       -- headless bootstrap (downloads mini.test, sets runtimepath)
  test_ansi.lua          -- ANSI/SGR parsing (jj.lua)
  test_file_tree.lua     -- tree building, compaction, sorting (file_tree.lua)
  test_file_list.lua     -- file list formatting and rendering (file_list.lua)
  test_utils.lua         -- utils.file_path, utils.await_all
  test_log_screen.lua    -- log screen layout & keymaps (Tier 2)
  test_review.lua        -- review screen lifecycle (Tier 2)
  TEST_PLAN.md           -- this file
```

## Tier 1 — Pure Logic

Testable functions are exposed via `M._test` tables on each module. These are
internal-only APIs used exclusively by tests.

### jj.lua — ANSI Parsing

Functions under test: `parse_ansi_line`, `ansi_256_to_hex`, `strip_ansi`

| Test Case                | Description                                                            |
| ------------------------ | ---------------------------------------------------------------------- |
| Reset code               | `\x1b[0m` resets all styles                                            |
| Standard fg colors       | `\x1b[31m` (red) through `\x1b[37m` produce correct hex                |
| Bright fg colors         | `\x1b[90m` through `\x1b[97m` map to palette indices 8-15              |
| 256-color fg             | `\x1b[38;5;Nm` for standard (0-15), cube (16-231), grayscale (232-255) |
| 24-bit RGB fg            | `\x1b[38;2;R;G;Bm` produces `#RRGGBB`                                  |
| Background colors        | Standard, bright, 256-color, 24-bit bg variants                        |
| Bold                     | `\x1b[1m` sets bold, `\x1b[22m` clears it                              |
| Nested styles            | Multiple SGR codes in one sequence (e.g. `\x1b[1;31m`)                 |
| Style spans              | Highlight spans have correct byte offsets                              |
| Malformed sequences      | Incomplete ESC sequences are passed through as text                    |
| Plain text passthrough   | Text without ANSI codes returns unchanged with no highlights           |
| `\x01` marker extraction | Commit header lines split correctly at `\x01` boundary                 |

### file_tree.lua — Tree Building

Functions under test: `build_tree`, `review_indicator`, `status_indicator`,
`format_file_line`, `format_dir_line`

| Test Case                            | Description                                                                                                 |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Single file at root                  | `["foo.lua"]` → one file node                                                                               |
| Nested directory                     | `["a/b/c.lua"]` → directory chain with file leaf                                                            |
| Single-child compaction              | `a/b/c.lua` where `a` and `b` each have one child → compacted to `a/b` dir                                  |
| No compaction when multiple children | Dir with 2+ children is not compacted                                                                       |
| Sort order                           | Directories before files, alphabetical within each group                                                    |
| review_indicator                     | Maps `reviewed`→`[x]`, `partiallyReviewed`→`[~]`, `reviewedReverted`→`[!]`, other→`[ ]`                     |
| status_indicator                     | Maps `added`→`A`, `modified`→`M`, `deleted`→`D`, `renamed`→`R`, `copied`→`C`, `typechange`→`T`, unknown→`?` |
| format_file_line                     | Correct text layout: indent + indicator + name + status + stats                                             |
| format_dir_line                      | Correct indent and highlight group                                                                          |

### file_list.lua — File List Formatting

Functions under test: `format_file_line`, `count_reviewed`, `render`

| Test Case                   | Description                                               |
| --------------------------- | --------------------------------------------------------- |
| count_reviewed              | Counts only files with `reviewStatus == "reviewed"`       |
| format_file_line layout     | `[x]  path/to/file M +5 -3` with correct column offsets   |
| Stats omitted when zero     | No `+0 -0` suffix when additions and deletions are both 0 |
| Review indicator highlights | Correct hl_group for each review status                   |
| render header               | First line reads ` Files N/M`                             |
| render cursor position      | Cursor placed on `selected_index + 2` (header + blank)    |

### utils.lua

| Test Case                   | Description                                                    |
| --------------------------- | -------------------------------------------------------------- |
| file_path with newPath      | Returns `newPath` when present                                 |
| file_path with oldPath only | Returns `oldPath` when `newPath` is nil                        |
| file_path assertion         | Errors when both are nil                                       |
| await_all success           | Collects results from multiple tasks into `{key = result}` map |
| await_all error             | First error short-circuits; callback receives error            |
| await_all empty             | Empty task table calls callback immediately with empty results |

## Tier 2 — Neovim API Integration

These tests run inside headless Neovim and verify buffer/window/keymap behavior.
`kjn.run` and `jj.log` are mocked at the module level to return canned responses.

### Log Screen

| Test Case          | Description                                                            |
| ------------------ | ---------------------------------------------------------------------- |
| Tab creation       | `:Kenjutu log` opens a new tab                                         |
| Buffer filetype    | Log buffer has `filetype = "kenjutu-log"`                              |
| File tree sidebar  | A second window opens in the tab with `filetype = "kenjutu-log-files"` |
| Keymaps registered | `q`, `j`, `k`, `<CR>`, `r` are mapped on the log buffer                |
| `q` closes tab     | Pressing `q` closes the tab and returns to previous                    |

### Review Screen

| Test Case          | Description                                               |
| ------------------ | --------------------------------------------------------- |
| Three-pane layout  | Opening review creates file list window + 2 diff windows  |
| File list filetype | File list buffer has `filetype = "kenjutu-review-files"`  |
| Close restores log | Pressing `q` in review returns to the log screen          |
| Toggle diff mode   | `t` toggles between "remaining" and "reviewed" diff state |

### File List Rendering (integration)

| Test Case                | Description                                    |
| ------------------------ | ---------------------------------------------- |
| Buffer content           | `render()` writes expected lines to buffer     |
| Extmarks applied         | Highlight extmarks exist at expected positions |
| Buffer is not modifiable | `modifiable` is false after render             |

## Mocking Strategy

Tier 2 tests mock at the module level:

```lua
-- Replace kjn.run with a stub
local kjn = require("kenjutu.kjn")
kjn.run = function(dir, args, callback)
  -- Return canned JSON for known commands
end
```

This avoids needing real `jj` repos or the `kjn` binary during tests while still
exercising all Neovim API interactions (buffer creation, window layout, keymaps,
extmarks).

## CI Integration

The `lua` job in `.github/workflows/ci.yml` gains:

1. Install Neovim (stable) from GitHub releases
2. Run `make test-lua`

Tests run after the existing `stylua --check` and `lua-language-server --check` steps.
