local kjn = require("kenjutu.kjn")
local diff = require("kenjutu.diff")
local file_list = require("kenjutu.file_list")
local utils = require("kenjutu.utils")

local M = {}

---@class kenjutu.ReviewState
---@field dir string
---@field change_id string
---@field commit_id string
---@field files kenjutu.FileEntry[]
---@field selected_index integer 1-indexed
---@field file_list_bufnr integer
---@field file_list_winnr integer
---@field diff_state kenjutu.DiffState  persistent diff pane state
---@field log_bufnr integer
local ReviewState = {}
ReviewState.__index = ReviewState

---@param opts { dir: string, change_id: string, commit_id: string, file_list_bufnr: integer, file_list_winnr: integer, diff_state: kenjutu.DiffState, log_bufnr: integer }
---@return kenjutu.ReviewState
function ReviewState.new(opts)
  --- @type kenjutu.ReviewState
  local fields = {
    dir = opts.dir,
    change_id = opts.change_id,
    commit_id = opts.commit_id,
    files = {},
    selected_index = 1,
    file_list_bufnr = opts.file_list_bufnr,
    file_list_winnr = opts.file_list_winnr,
    diff_state = opts.diff_state,
    log_bufnr = opts.log_bufnr,
  }
  local self = setmetatable(fields, ReviewState)
  return self
end

--- Load and display the diff for the currently selected file.
function ReviewState:update_diff_view()
  if #self.files == 0 then
    return
  end
  local file = self.files[self.selected_index]
  if not file then
    return
  end
  self.diff_state:set_file(file, self.dir, self.change_id, self.commit_id)
end

---@return fun(bufnr: integer)
function ReviewState:make_diff_keymap_installer()
  return function(bufnr)
    local opts = { buffer = bufnr, silent = true }

    -- Tab: focus back to file list
    vim.keymap.set("n", "<Tab>", function()
      if vim.api.nvim_win_is_valid(self.file_list_winnr) then
        vim.api.nvim_set_current_win(self.file_list_winnr)
      end
    end, opts)

    -- Space: mark/unmark the hunk under cursor
    vim.keymap.set("n", "<Space>", function()
      local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
      local result = self.diff_state:resolve_hunk(bufnr, cursor_line)
      if not result then
        return
      end
      self:toggle_hunk_reviewed(result.hunk, result.action)
    end, opts)

    vim.keymap.set("n", "gj", function()
      self:move_selection(1)
    end, opts)

    vim.keymap.set("n", "gk", function()
      self:move_selection(-1)
    end, opts)

    -- q: close review entirely
    vim.keymap.set("n", "q", function()
      self:close()
    end, opts)
  end
end

--- Move file selection by delta and load the diff.
---@param delta integer
function ReviewState:move_selection(delta)
  if #self.files == 0 then
    return
  end
  local new_index = self.selected_index + delta
  if new_index < 1 then
    new_index = 1
  elseif new_index > #self.files then
    new_index = #self.files
  end
  if new_index == self.selected_index then
    return
  end
  self.selected_index = new_index
  local target_line = new_index + 2 -- header + blank
  if vim.api.nvim_win_is_valid(self.file_list_winnr) then
    vim.api.nvim_win_set_cursor(self.file_list_winnr, { target_line, 0 })
  end
  self:update_diff_view()
end

--- Refresh the file list by re-running kjn files, then reload the diff.
function ReviewState:refresh_file_list()
  kjn.run(self.dir, { "files", "--change-id", self.change_id }, function(err, result)
    if err then
      vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
      return
    end
    if not result or not vim.api.nvim_buf_is_valid(self.file_list_bufnr) then
      return
    end
    assert(type(result.commitId) == "string", "missing commitId in kjn files result")
    self.commit_id = result.commitId
    self.files = result.files or {}
    if self.selected_index > #self.files then
      self.selected_index = math.max(1, #self.files)
    end
    file_list.render(self.file_list_bufnr, self.files, self.selected_index, self.file_list_winnr)
    self:update_diff_view()
  end)
end

---@param file kenjutu.FileEntry
---@return string[]
function ReviewState:mark_args(file)
  local path = utils.file_path(file)
  local old_path = file.oldPath
  local new_path = file.newPath
  local args = {
    "--change-id",
    self.change_id,
    "--commit",
    self.commit_id,
    "--file",
    path,
  }
  if old_path and new_path and old_path ~= new_path then
    table.insert(args, "--old-path")
    table.insert(args, file.oldPath)
  end
  return args
end

---@param hunk {old_start: integer, old_lines: integer, new_start: integer, new_lines: integer}
---@param action DiffAction
function ReviewState:toggle_hunk_reviewed(hunk, action)
  if #self.files == 0 then
    return
  end
  local file = self.files[self.selected_index]
  if not file then
    return
  end

  local subcmd = action == "mark" and "mark-hunk" or "unmark-hunk"
  local args = { subcmd }
  for _, a in ipairs(self:mark_args(file)) do
    table.insert(args, a)
  end
  table.insert(args, "--old-start")
  table.insert(args, tostring(hunk.old_start))
  table.insert(args, "--old-lines")
  table.insert(args, tostring(hunk.old_lines))
  table.insert(args, "--new-start")
  table.insert(args, tostring(hunk.new_start))
  table.insert(args, "--new-lines")
  table.insert(args, tostring(hunk.new_lines))

  kjn.run(self.dir, args, function(err, _)
    if err then
      vim.notify("kjn " .. subcmd .. ": " .. err, vim.log.levels.ERROR)
      return
    end
    self:refresh_file_list()
  end)
end

function ReviewState:toggle_file_reviewed()
  if #self.files == 0 then
    return
  end
  local file = self.files[self.selected_index]
  local subcmd = file.reviewStatus == "reviewed" and "unmark-file" or "mark-file"
  local args = { subcmd }
  for _, a in ipairs(self:mark_args(file)) do
    table.insert(args, a)
  end

  kjn.run(self.dir, args, function(err, _)
    if err then
      vim.notify("kjn " .. subcmd .. ": " .. err, vim.log.levels.ERROR)
      return
    end
    self:refresh_file_list()
  end)
end

--- Close the review screen and restore the log buffer.
function ReviewState:close()
  local log_bufnr = self.log_bufnr
  local file_list_bufnr = self.file_list_bufnr

  M._state[file_list_bufnr] = nil

  -- Close diff windows
  local anchor_winnr = self.diff_state.anchor_winnr
  self.diff_state:close()

  if vim.api.nvim_win_is_valid(anchor_winnr) then
    vim.api.nvim_win_close(anchor_winnr, true)
  end

  -- The file list window should now be the only window in the tab.
  -- Switch it to show the log buffer.
  local win = vim.api.nvim_get_current_win()
  if vim.api.nvim_buf_is_valid(log_bufnr) then
    vim.api.nvim_win_set_buf(win, log_bufnr)
    vim.wo[win].cursorline = true
    vim.wo[win].number = false
    vim.wo[win].relativenumber = false
    vim.wo[win].signcolumn = "no"
    vim.wo[win].wrap = false
    vim.wo[win].winfixwidth = false
  end

  if vim.api.nvim_buf_is_valid(file_list_bufnr) then
    vim.api.nvim_buf_delete(file_list_bufnr, { force = true })
  end
end

function ReviewState:setup_file_list_keymaps()
  local bufnr = self.file_list_bufnr
  local opts = { buffer = bufnr, silent = true }

  vim.keymap.set("n", "j", function()
    self:move_selection(1)
  end, opts)

  vim.keymap.set("n", "k", function()
    self:move_selection(-1)
  end, opts)

  vim.keymap.set("n", "<CR>", function()
    local s = M._state[bufnr]
    if s and s.diff_state and s.diff_state.pane then
      if vim.api.nvim_win_is_valid(s.diff_state.pane.right_winnr) then
        vim.api.nvim_set_current_win(s.diff_state.pane.right_winnr)
      end
    end
  end, opts)

  vim.keymap.set("n", "<Space>", function()
    self:toggle_file_reviewed()
  end, opts)

  vim.keymap.set("n", "r", function()
    self:refresh_file_list()
  end, opts)

  vim.keymap.set("n", "q", function()
    self:close()
  end, opts)
end

--- Per-buffer state registry keyed by file list buffer number.
---@type table<integer, kenjutu.ReviewState>
M._state = {}

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
  local file_list_bufnr = create_scratch_buf("kenjutu-review-files")

  -- Set up layout: replace current window with file list, open diff anchor split
  local cur_win = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(cur_win, file_list_bufnr)

  vim.cmd("rightbelow vsplit")
  local diff_anchor_winnr = vim.api.nvim_get_current_win()

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

  -- Show loading state in file list
  vim.bo[file_list_bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(file_list_bufnr, 0, -1, false, { "Loading..." })
  vim.bo[file_list_bufnr].modifiable = false

  -- Create ReviewState first (needed for keymap installer closure)
  local s = ReviewState.new({
    dir = dir,
    change_id = commit.change_id,
    commit_id = commit.commit_id,
    file_list_bufnr = file_list_bufnr,
    file_list_winnr = file_list_winnr,
    log_bufnr = log_bufnr,
  })

  s.diff_state = diff.create({
    anchor_winnr = diff_anchor_winnr,
    setup_keymaps = s:make_diff_keymap_installer(),
  })

  -- Restore focus to file list after diff layout creation
  vim.api.nvim_set_current_win(file_list_winnr)
  vim.api.nvim_win_set_width(file_list_winnr, 40)

  M._state[file_list_bufnr] = s

  s:setup_file_list_keymaps()

  -- Fetch file list
  kjn.run(dir, { "files", "--change-id", commit.change_id }, function(err, result)
    if err then
      vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
      return
    end
    if not result or not vim.api.nvim_buf_is_valid(file_list_bufnr) then
      return
    end

    assert(type(result.commitId) == "string", "missing commitId in kjn files result")
    s.commit_id = result.commitId
    s.files = result.files or {}
    s.selected_index = #s.files > 0 and 1 or 0

    file_list.render(s.file_list_bufnr, s.files, s.selected_index, s.file_list_winnr)
    s:update_diff_view()
  end)

  -- Clean up state when buffer is wiped
  vim.api.nvim_create_autocmd("BufWipeout", {
    buffer = file_list_bufnr,
    once = true,
    callback = function()
      M._state[file_list_bufnr] = nil
    end,
  })
end

return M
