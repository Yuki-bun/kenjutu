local jj = require("kenjutu.jj")

local M = {}

---@class kenjutu.LogScreenState
---@field commits_by_line table<integer, kenjutu.Commit>
---@field commit_lines integer[]
---@field dir string

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

--- Render parsed jj log output into the buffer.
---@param bufnr integer
---@param result kenjutu.LogResult
---@param restore_change_id string|nil  change_id to restore cursor to
local function render(bufnr, result, restore_change_id)
  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, result.lines)
  vim.bo[bufnr].modifiable = false

  state[bufnr] = {
    commits_by_line = result.commits_by_line,
    commit_lines = result.commit_lines,
    dir = state[bufnr] and state[bufnr].dir or vim.fn.getcwd(),
  }

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

  -- Enter: stub — show selected commit info
  vim.keymap.set("n", "<CR>", function()
    local s = state[bufnr]
    if not s then
      return
    end
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local commit = s.commits_by_line[cur]
    if commit then
      vim.notify("Review: " .. commit.change_id, vim.log.levels.INFO)
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
    local tab_count = #vim.api.nvim_list_tabpages()
    if tab_count > 1 then
      vim.cmd("tabclose")
    else
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
    state[bufnr] = nil
  end, opts)
end

--- Open the commit log screen in a new tab.
function M.open()
  vim.cmd("tabnew")
  local bufnr = vim.api.nvim_get_current_buf()
  local winnr = vim.api.nvim_get_current_win()

  -- Buffer options
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-log"

  -- Window options
  vim.wo[winnr].cursorline = true
  vim.wo[winnr].number = false
  vim.wo[winnr].relativenumber = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false

  -- Show loading state
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "Loading..." })
  vim.bo[bufnr].modifiable = false

  local dir = vim.fn.getcwd()
  state[bufnr] = { commits_by_line = {}, commit_lines = {}, dir = dir }

  setup_keymaps(bufnr)

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
      state[bufnr] = nil
    end,
  })
end

return M
