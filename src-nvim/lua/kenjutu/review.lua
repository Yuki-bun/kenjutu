local kjn = require("kenjutu.kjn")

local M = {}

---@class kenjutu.FileEntry
---@field oldPath string|nil
---@field newPath string|nil
---@field status string "added"|"modified"|"deleted"|"renamed"|"copied"|"typechange"
---@field additions integer
---@field deletions integer
---@field isBinary boolean
---@field reviewStatus string "reviewed"|"partiallyReviewed"|"unreviewed"|"reviewedReverted"

---@class kenjutu.ReviewState
---@field dir string
---@field change_id string
---@field commit_id string
---@field files kenjutu.FileEntry[]
---@field selected_index integer 1-indexed
---@field file_list_bufnr integer
---@field diff_bufnr integer
---@field file_list_winnr integer
---@field diff_winnr integer
---@field log_bufnr integer

--- Per-buffer state keyed by file list buffer number.
---@type table<integer, kenjutu.ReviewState>
local state = {}

local ns = vim.api.nvim_create_namespace("kenjutu_review")

-- Highlight groups ----------------------------------------------------------

local hl_defs = {
  KenjutuReviewed = { fg = "#a6e3a1" },
  KenjutuPartial = { fg = "#f9e2af" },
  KenjutuReverted = { fg = "#6c7086" },
  KenjutuStatusA = { fg = "#a6e3a1" },
  KenjutuStatusM = { fg = "#f9e2af" },
  KenjutuStatusD = { fg = "#f38ba8" },
  KenjutuStatusR = { fg = "#89b4fa" },
  KenjutuStatusC = { fg = "#94e2d5" },
  KenjutuStatusT = { fg = "#cba6f7" },
  KenjutuStats = { fg = "#6c7086" },
  KenjutuHeader = { fg = "#cdd6f4", bold = true },
}

for name, def in pairs(hl_defs) do
  vim.api.nvim_set_hl(0, name, def)
end

-- Helpers -------------------------------------------------------------------

--- Return the display path for a file entry.
---@param file kenjutu.FileEntry
---@return string
local function file_path(file)
  return file.newPath or file.oldPath or "<unknown>"
end

--- Map review status to bracket indicator and highlight group.
---@param status string
---@return string indicator
---@return string|nil hl_group
local function review_indicator(status)
  if status == "reviewed" then
    return "[x]", "KenjutuReviewed"
  elseif status == "partiallyReviewed" then
    return "[~]", "KenjutuPartial"
  elseif status == "reviewedReverted" then
    return "[!]", "KenjutuReverted"
  else
    return "[ ]", nil
  end
end

--- Map file change status to a letter and highlight group.
---@param status string
---@return string letter
---@return string hl_group
local function status_indicator(status)
  local map = {
    added = { "A", "KenjutuStatusA" },
    modified = { "M", "KenjutuStatusM" },
    deleted = { "D", "KenjutuStatusD" },
    renamed = { "R", "KenjutuStatusR" },
    copied = { "C", "KenjutuStatusC" },
    typechange = { "T", "KenjutuStatusT" },
  }
  local entry = map[status]
  if entry then
    return entry[1], entry[2]
  end
  return "?", "KenjutuStats"
end

--- Count reviewed files.
---@param files kenjutu.FileEntry[]
---@return integer
local function count_reviewed(files)
  local n = 0
  for _, f in ipairs(files) do
    if f.reviewStatus == "reviewed" then
      n = n + 1
    end
  end
  return n
end

-- Rendering -----------------------------------------------------------------

--- Build a plain text line for one file entry.
--- Returns the line string and a list of {col_start, col_end, hl_group} tuples.
---@param file kenjutu.FileEntry
---@return string line
---@return {[1]: integer, [2]: integer, [3]: string}[] highlights
local function format_file_line(file)
  local indicator, indicator_hl = review_indicator(file.reviewStatus)
  local path = file_path(file)
  local status_char, status_hl = status_indicator(file.status)

  -- Build the line:  "[x]  path M +N -M"
  local parts = {}
  local highlights = {}
  local col = 0

  -- Review indicator
  table.insert(parts, indicator)
  if indicator_hl then
    table.insert(highlights, { col, col + #indicator, indicator_hl })
  end
  col = col + #indicator

  -- Two spaces separator
  table.insert(parts, "  ")
  col = col + 2

  -- File path (default color)
  table.insert(parts, path)
  col = col + #path

  -- Space + status letter
  local status_str = " " .. status_char
  table.insert(parts, status_str)
  -- Highlight just the letter, not the space
  table.insert(highlights, { col + 1, col + 1 + #status_char, status_hl })
  col = col + #status_str

  -- Stats
  if file.additions > 0 or file.deletions > 0 then
    local stats = ""
    if file.additions > 0 then
      stats = stats .. " +" .. file.additions
    end
    if file.deletions > 0 then
      stats = stats .. " -" .. file.deletions
    end
    table.insert(parts, stats)
    table.insert(highlights, { col, col + #stats, "KenjutuStats" })
    col = col + #stats
  end

  return table.concat(parts), highlights
end

--- Render the file list into the buffer.
---@param s kenjutu.ReviewState
local function render_file_list(s)
  local bufnr = s.file_list_bufnr
  local lines = {}
  local all_highlights = {} -- [line_index] = highlights

  -- Header line
  local reviewed = count_reviewed(s.files)
  local header = string.format(" Files %d/%d", reviewed, #s.files)
  table.insert(lines, header)
  table.insert(all_highlights, { { 0, #header, "KenjutuHeader" } })

  -- Blank separator
  table.insert(lines, "")
  table.insert(all_highlights, {})

  -- File lines (1-indexed file index maps to line index - 2)
  for _, file in ipairs(s.files) do
    local line, highlights = format_file_line(file)
    table.insert(lines, line)
    table.insert(all_highlights, highlights)
  end

  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false

  -- Apply extmark highlights
  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  for i, highlights in ipairs(all_highlights) do
    for _, hl in ipairs(highlights) do
      vim.api.nvim_buf_set_extmark(bufnr, ns, i - 1, hl[1], { end_col = hl[2], hl_group = hl[3] })
    end
  end

  -- Position cursor on selected file (account for header + blank line)
  if s.selected_index >= 1 and s.selected_index <= #s.files then
    local target_line = s.selected_index + 2 -- header + blank
    if vim.api.nvim_win_is_valid(s.file_list_winnr) then
      vim.api.nvim_win_set_cursor(s.file_list_winnr, { target_line, 0 })
    end
  end
end

--- Update the diff placeholder to show the selected file name.
---@param s kenjutu.ReviewState
local function update_diff_placeholder(s)
  if not vim.api.nvim_buf_is_valid(s.diff_bufnr) then
    return
  end
  local msg
  if #s.files == 0 then
    msg = "No changed files"
  else
    local file = s.files[s.selected_index]
    msg = "Diff view not yet implemented"
        .. "\n\n"
        .. "Selected: "
        .. file_path(file)
  end
  local lines = vim.split(msg, "\n", { plain = true })
  vim.bo[s.diff_bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(s.diff_bufnr, 0, -1, false, lines)
  vim.bo[s.diff_bufnr].modifiable = false
end

-- Navigation ----------------------------------------------------------------

---@param s kenjutu.ReviewState
---@param delta integer +1 or -1
local function move_selection(s, delta)
  if #s.files == 0 then
    return
  end
  local new_index = s.selected_index + delta
  if new_index < 1 then
    new_index = 1
  elseif new_index > #s.files then
    new_index = #s.files
  end
  if new_index == s.selected_index then
    return
  end
  s.selected_index = new_index
  -- Move cursor to the corresponding line
  local target_line = new_index + 2 -- header + blank
  if vim.api.nvim_win_is_valid(s.file_list_winnr) then
    vim.api.nvim_win_set_cursor(s.file_list_winnr, { target_line, 0 })
  end
  update_diff_placeholder(s)
end

-- File marking --------------------------------------------------------------

--- Refresh the file list by re-running kjn files.
---@param s kenjutu.ReviewState
local refresh_file_list = function(s)
  kjn.run(s.dir, { "files", "--commit", s.commit_id }, function(err, result)
    if err then
      vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
      return
    end
    if not result or not vim.api.nvim_buf_is_valid(s.file_list_bufnr) then
      return
    end
    s.change_id = result.changeId or s.change_id
    s.files = result.files or {}
    -- Clamp selected index
    if s.selected_index > #s.files then
      s.selected_index = math.max(1, #s.files)
    end
    render_file_list(s)
    update_diff_placeholder(s)
  end)
end

--- Build kjn args for mark/unmark commands.
---@param s kenjutu.ReviewState
---@param file kenjutu.FileEntry
---@return string[] base_args  common args (without the subcommand)
local function mark_args(s, file)
  local path = file_path(file)
  local args = {
    "--change-id",
    s.change_id,
    "--commit",
    s.commit_id,
    "--file",
    path,
  }
  -- For renames/copies, supply old-path
  if file.oldPath and file.newPath and file.oldPath ~= file.newPath then
    table.insert(args, "--old-path")
    table.insert(args, file.oldPath)
  end
  return args
end

--- Toggle reviewed status for the currently selected file.
---@param s kenjutu.ReviewState
local function toggle_file_reviewed(s)
  if #s.files == 0 then
    return
  end
  local file = s.files[s.selected_index]
  local subcmd = file.reviewStatus == "reviewed" and "unmark-file" or "mark-file"
  local args = { subcmd }
  for _, a in ipairs(mark_args(s, file)) do
    table.insert(args, a)
  end

  kjn.run(s.dir, args, function(err, _)
    if err then
      vim.notify("kjn " .. subcmd .. ": " .. err, vim.log.levels.ERROR)
      return
    end
    -- Refresh file list to get updated review statuses
    refresh_file_list(s)
  end)
end


-- Layout & keymaps ----------------------------------------------------------

--- Set up keymaps for the file list buffer.
---@param bufnr integer
local function setup_keymaps(bufnr)
  local opts = { buffer = bufnr, silent = true }

  -- j: next file
  vim.keymap.set("n", "j", function()
    local s = state[bufnr]
    if s then
      move_selection(s, 1)
    end
  end, opts)

  -- k: previous file
  vim.keymap.set("n", "k", function()
    local s = state[bufnr]
    if s then
      move_selection(s, -1)
    end
  end, opts)

  -- Space: toggle file reviewed
  vim.keymap.set("n", "<Space>", function()
    local s = state[bufnr]
    if s then
      toggle_file_reviewed(s)
    end
  end, opts)

  -- r: refresh
  vim.keymap.set("n", "r", function()
    local s = state[bufnr]
    if s then
      refresh_file_list(s)
    end
  end, opts)

  -- q: close review, restore log screen
  vim.keymap.set("n", "q", function()
    local s = state[bufnr]
    if not s then
      return
    end
    local log_bufnr = s.log_bufnr
    local diff_bufnr_to_close = s.diff_bufnr
    state[bufnr] = nil

    -- Close the diff window (this wipes the diff buffer via bufhidden=wipe)
    if vim.api.nvim_win_is_valid(s.diff_winnr) then
      vim.api.nvim_win_close(s.diff_winnr, true)
    end

    -- The file list window should now be the only window in the tab.
    -- Switch it to show the log buffer, then clean up the file list buffer.
    local win = vim.api.nvim_get_current_win()
    if vim.api.nvim_buf_is_valid(log_bufnr) then
      vim.api.nvim_win_set_buf(win, log_bufnr)
      -- Restore log window options
      vim.wo[win].cursorline = true
      vim.wo[win].number = false
      vim.wo[win].relativenumber = false
      vim.wo[win].signcolumn = "no"
      vim.wo[win].wrap = false
      vim.wo[win].winfixwidth = false
    end

    -- Clean up leftover buffers
    if vim.api.nvim_buf_is_valid(bufnr) then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
    if vim.api.nvim_buf_is_valid(diff_bufnr_to_close) then
      vim.api.nvim_buf_delete(diff_bufnr_to_close, { force = true })
    end
  end, opts)
end

--- Create a scratch buffer with given filetype.
---@param ft string
---@return integer bufnr
local function create_scratch_buf(ft)
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = ft
  return bufnr
end

--- Open the review screen for a commit.
---@param dir string working directory
---@param commit kenjutu.Commit {change_id, commit_id}
---@param log_bufnr integer the log buffer to restore on q
function M.open(dir, commit, log_bufnr)
  -- Create file list buffer
  local file_list_bufnr = create_scratch_buf("kenjutu-review-files")
  -- Create diff placeholder buffer
  local diff_bufnr = create_scratch_buf("kenjutu-review-diff")

  -- Set up layout: replace current window with file list, open diff split
  local cur_win = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(cur_win, file_list_bufnr)

  -- Open vertical split for diff on the right
  vim.cmd("rightbelow vsplit")
  local diff_winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(diff_winnr, diff_bufnr)

  -- Set file list window as current and configure width
  local file_list_winnr = cur_win
  vim.api.nvim_set_current_win(file_list_winnr)
  vim.api.nvim_win_set_width(file_list_winnr, 40)

  -- File list window options
  vim.wo[file_list_winnr].cursorline = true
  vim.wo[file_list_winnr].number = false
  vim.wo[file_list_winnr].relativenumber = false
  vim.wo[file_list_winnr].signcolumn = "no"
  vim.wo[file_list_winnr].wrap = false
  vim.wo[file_list_winnr].winfixwidth = true

  -- Diff window options
  vim.wo[diff_winnr].number = false
  vim.wo[diff_winnr].relativenumber = false
  vim.wo[diff_winnr].signcolumn = "no"
  vim.wo[diff_winnr].wrap = true

  -- Show loading state
  vim.bo[file_list_bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(file_list_bufnr, 0, -1, false, { "Loading..." })
  vim.bo[file_list_bufnr].modifiable = false

  -- Show placeholder in diff window
  vim.bo[diff_bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(diff_bufnr, 0, -1, false, { "Diff view not yet implemented" })
  vim.bo[diff_bufnr].modifiable = false

  -- Initialize state
  ---@type kenjutu.ReviewState
  local s = {
    dir = dir,
    change_id = commit.change_id,
    commit_id = commit.commit_id,
    files = {},
    selected_index = 1,
    file_list_bufnr = file_list_bufnr,
    diff_bufnr = diff_bufnr,
    file_list_winnr = file_list_winnr,
    diff_winnr = diff_winnr,
    log_bufnr = log_bufnr,
  }
  state[file_list_bufnr] = s

  setup_keymaps(file_list_bufnr)

  -- Fetch file list
  kjn.run(dir, { "files", "--commit", commit.commit_id }, function(err, result)
    if err then
      vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
      return
    end
    if not result or not vim.api.nvim_buf_is_valid(file_list_bufnr) then
      return
    end

    s.change_id = result.changeId or s.change_id
    s.files = result.files or {}
    s.selected_index = #s.files > 0 and 1 or 0

    render_file_list(s)
    update_diff_placeholder(s)
  end)

  -- Clean up state when buffer is wiped
  vim.api.nvim_create_autocmd("BufWipeout", {
    buffer = file_list_bufnr,
    once = true,
    callback = function()
      state[file_list_bufnr] = nil
    end,
  })
end

return M
