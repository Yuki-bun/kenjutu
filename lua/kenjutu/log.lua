local jj = require("kenjutu.jj")
local file_tree = require("kenjutu.file_tree")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_log")

---@class kenjutu.LogScreenState
---@field commits_by_line table<integer, kenjutu.Commit>
---@field commit_lines integer[]
---@field dir string
---@field file_tree kenjutu.FileTreeState|nil

--- Per-buffer state for log screens.
---@type table<integer, kenjutu.LogScreenState>
local state = {}

--- Find the next commit line after `current` (1-indexed).
---@param commit_lines integer[]
---@param current integer
---@return integer|nil
local function next_commit_line(commit_lines, current)
  for _, line_no in ipairs(commit_lines) do
    if line_no > current then
      return line_no
    end
  end
  return nil
end

--- Find the previous commit line before `current` (1-indexed).
---@param commit_lines integer[]
---@param current integer
---@return integer|nil
local function prev_commit_line(commit_lines, current)
  local result = nil
  for _, line_no in ipairs(commit_lines) do
    if line_no < current then
      result = line_no
    else
      break
    end
  end
  return result
end

--- Find the commit at or nearest before the cursor position.
---@param s kenjutu.LogScreenState
---@param cursor_line integer
---@return kenjutu.Commit|nil
local function commit_at_cursor(s, cursor_line)
  if s.commits_by_line[cursor_line] then
    return s.commits_by_line[cursor_line]
  end
  local nearest = nil
  for _, line_no in ipairs(s.commit_lines) do
    if line_no <= cursor_line then
      nearest = line_no
    else
      break
    end
  end
  if nearest then
    return s.commits_by_line[nearest]
  end
  return nil
end

--- Render parsed jj log output into the buffer with syntax highlighting.
---@param bufnr integer
---@param result kenjutu.LogResult
---@param restore_change_id string|nil  change_id to restore cursor to
local function render(bufnr, result, restore_change_id)
  local s = state[bufnr]

  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, result.lines)
  vim.bo[bufnr].modifiable = false

  state[bufnr] = {
    commits_by_line = result.commits_by_line,
    commit_lines = result.commit_lines,
    dir = s and s.dir or vim.fn.getcwd(),
    file_tree = s and s.file_tree or nil,
  }

  -- Apply extmark highlights from parsed ANSI data
  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  if result.highlights then
    for line_idx, spans in pairs(result.highlights) do
      for _, span in ipairs(spans) do
        pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, line_idx - 1, span.col_start, {
          end_col = span.col_end,
          hl_group = span.hl_group,
        })
      end
    end
  end

  -- Position cursor
  local target_line = result.commit_lines[1]
  if restore_change_id then
    for _, line_no in ipairs(result.commit_lines) do
      if result.commits_by_line[line_no].change_id == restore_change_id then
        target_line = line_no
        break
      end
    end
  end
  if target_line then
    vim.api.nvim_win_set_cursor(0, { target_line, 0 })
  end
end

--- Set up buffer-local keymaps.
---@param bufnr integer
local function setup_keymaps(bufnr)
  local opts = { buffer = bufnr, silent = true }

  -- j: next commit line
  vim.keymap.set("n", "j", function()
    local s = state[bufnr]
    if not s then
      return
    end
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local target = next_commit_line(s.commit_lines, cur)
    if target then
      vim.api.nvim_win_set_cursor(0, { target, 0 })
    end
  end, opts)

  -- k: previous commit line
  vim.keymap.set("n", "k", function()
    local s = state[bufnr]
    if not s then
      return
    end
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local target = prev_commit_line(s.commit_lines, cur)
    if target then
      vim.api.nvim_win_set_cursor(0, { target, 0 })
    end
  end, opts)

  -- Enter: open review screen for the selected commit
  vim.keymap.set("n", "<CR>", function()
    local s = state[bufnr]
    if not s then
      return
    end
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local commit = s.commits_by_line[cur]
    if commit then
      if s.file_tree then
        s.file_tree:close()
        s.file_tree = nil
      end
      require("kenjutu.review").open(s.dir, commit, bufnr)
    end
  end, opts)

  -- r: refresh
  vim.keymap.set("n", "r", function()
    local s = state[bufnr]
    if not s then
      return
    end
    -- Remember current commit for cursor restore
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local current_commit = s.commits_by_line[cur]
    local restore_id = current_commit and current_commit.change_id or nil

    jj.log(s.dir, function(err, result)
      if err or result == nil then
        vim.notify("jj log: " .. err, vim.log.levels.ERROR)
        return
      end
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end
      render(bufnr, result, restore_id)
    end)
  end, opts)

  -- q: close
  vim.keymap.set("n", "q", function()
    local s = state[bufnr]
    if s then
      if s.file_tree then
        s.file_tree:close()
        s.file_tree = nil
      end
    end
    state[bufnr] = nil
    local tab_count = #vim.api.nvim_list_tabpages()
    if tab_count > 1 then
      vim.cmd("tabclose")
    end
    if vim.api.nvim_buf_is_valid(bufnr) then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
  end, opts)
end

--- Set up autocmd to update the file tree when the cursor moves.
---@param bufnr integer
local function setup_cursor_follow(bufnr)
  vim.api.nvim_create_autocmd("CursorMoved", {
    buffer = bufnr,
    callback = function()
      local s = state[bufnr]
      if not s then
        return
      end
      local cur = vim.api.nvim_win_get_cursor(0)[1]
      local commit = commit_at_cursor(s, cur)
      if not commit then
        return
      end

      if not s.file_tree then
        local log_winnr = vim.api.nvim_get_current_win()
        s.file_tree = file_tree.FileTreeState.new(s.dir, log_winnr)
      end
      s.file_tree:update(commit)
    end,
  })
end

--- Open the commit log screen in a new tab.
function M.open()
  vim.cmd("tabnew")
  local bufnr = vim.api.nvim_get_current_buf()
  local log_winnr = vim.api.nvim_get_current_win()

  -- Buffer options
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "hide"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-log"

  -- Window options
  vim.wo[log_winnr].cursorline = true
  vim.wo[log_winnr].number = false
  vim.wo[log_winnr].relativenumber = false
  vim.wo[log_winnr].signcolumn = "no"
  vim.wo[log_winnr].wrap = false

  -- Show loading state
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "Loading..." })
  vim.bo[bufnr].modifiable = false

  local dir = vim.fn.getcwd()
  state[bufnr] = {
    commits_by_line = {},
    commit_lines = {},
    dir = dir,
    file_tree = file_tree.FileTreeState.new(dir, log_winnr),
  }

  vim.api.nvim_set_current_win(log_winnr)

  setup_keymaps(bufnr)
  setup_cursor_follow(bufnr)

  jj.log(dir, function(err, result)
    if err or result == nil then
      vim.notify("jj log: " .. err, vim.log.levels.ERROR)
      return
    end
    if not vim.api.nvim_buf_is_valid(bufnr) then
      return
    end
    render(bufnr, result, nil)
  end)

  -- Clean up state when buffer is wiped
  vim.api.nvim_create_autocmd("BufWipeout", {
    buffer = bufnr,
    once = true,
    callback = function()
      local s = state[bufnr]
      if s and s.file_tree then
        s.file_tree:close()
        s.file_tree = nil
      end
      state[bufnr] = nil
    end,
  })
end

return M
