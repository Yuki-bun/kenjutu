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
---@field line_map table<integer, kenjutu.FileEntry>
---@field file_list_bufnr integer
---@field file_list_winnr integer
---@field diff_state kenjutu.DiffState  persistent diff pane state
---@field log_bufnr integer
---@field on_close function callback to run after review screen is closed
local ReviewState = {}
ReviewState.__index = ReviewState

---@class kenjutu.ReviewStateInitOpts
---@field dir string
---@field change_id string
---@field commit_id string
---@field file_list_bufnr integer
---@field file_list_winnr integer
---@field diff_state kenjutu.DiffState
---@field log_bufnr integer
---@field on_close function

---@param opts kenjutu.ReviewStateInitOpts
---@return kenjutu.ReviewState
function ReviewState.new(opts)
  --- @type kenjutu.ReviewState
  local fields = {
    dir = opts.dir,
    change_id = opts.change_id,
    commit_id = opts.commit_id,
    files = {},
    line_map = {},
    file_list_bufnr = opts.file_list_bufnr,
    diff_state = opts.diff_state,
    file_list_winnr = opts.file_list_winnr,
    log_bufnr = opts.log_bufnr,
    on_close = opts.on_close,
  }
  local self = setmetatable(fields, ReviewState)
  return self
end

function ReviewState:update_diff_view()
  local file = self:selected_file()
  if not file then
    return
  end
  self.diff_state:set_file(file)
end

--- Move file selection to the next file line in the given direction.
---@param direction "up" | "down"
function ReviewState:move_selection(direction)
  if #self.files == 0 or not vim.api.nvim_win_is_valid(self.file_list_winnr) then
    return
  end
  local cur_line = vim.api.nvim_win_get_cursor(self.file_list_winnr)[1]
  local line_count = vim.api.nvim_buf_line_count(self.file_list_bufnr)
  local step = direction == "down" and 1 or -1
  local line = cur_line + step
  while line >= 1 and line <= line_count do
    if self.line_map[line] then
      vim.api.nvim_win_set_cursor(self.file_list_winnr, { line, 0 })
      self:update_diff_view()
      return
    end
    line = line + step
  end
end

---@param file_path string
---@param line integer|nil
---@param side "New"|"Old"
function ReviewState:navigate_to(file_path, line, side)
  for line_no, file in pairs(self.line_map) do
    if utils.file_path(file) == file_path then
      self.diff_state:set_file(file, { line = line, side = side })
      vim.api.nvim_win_set_cursor(self.file_list_winnr, { line_no, 0 })
    end
  end
end

function ReviewState:refresh_file_list()
  kjn.files(self.dir, self.change_id, function(err, result)
    if err then
      vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
      return
    end
    if not result or not vim.api.nvim_buf_is_valid(self.file_list_bufnr) then
      return
    end
    local commit_changed = self.commit_id ~= result.commitId
    self.commit_id = result.commitId
    self.files = result.files or {}
    self.line_map = file_list.render(self.file_list_bufnr, self.files, self.file_list_winnr)
    if commit_changed then
      self.diff_state:reload(self.commit_id)
    end
  end)
end

function ReviewState:toggle_file_reviewed()
  local file = self:selected_file()
  if not file then
    return
  end
  local path = utils.file_path(file)
  local old_path = file.oldPath
  local new_path = file.newPath
  ---@type kenjutu.MarkFileOptions
  local opts = {
    dir = self.dir,
    change_id = self.change_id,
    commit_id = self.commit_id,
    file_path = path,
  }
  if old_path and new_path and old_path ~= new_path then
    opts.old_path = old_path
  end

  local fn = file.reviewStatus == "reviewed" and kjn.unmark_file or kjn.mark_file
  fn(opts, function(err, _)
    if err then
      vim.notify("kjn toggle-reviewed: " .. err, vim.log.levels.ERROR)
      return
    end
    self:refresh_file_list()
    local new_status = file.reviewStatus == "reviewed" and "unreviewed" or "reviewed"
    self.diff_state:on_file_toggled(file, new_status)
  end)
end

--- Close the review screen and restore the log buffer.
function ReviewState:close()
  local log_bufnr = self.log_bufnr
  local file_list_bufnr = self.file_list_bufnr

  -- Close diff windows
  local anchor_winnr = self.diff_state.left_winnr
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

  self.on_close()
end

function ReviewState:setup_file_list_keymaps()
  local bufnr = self.file_list_bufnr
  local opts = { buffer = bufnr, silent = true }

  vim.keymap.set("n", "<CR>", function()
    if vim.api.nvim_win_is_valid(self.diff_state.right_winnr) then
      vim.api.nvim_set_current_win(self.diff_state.right_winnr)
    end
  end, opts)

  vim.keymap.set("n", "<Space>", function()
    self:toggle_file_reviewed()
  end, vim.tbl_extend("force", opts, { nowait = true }))

  vim.keymap.set("n", "r", function()
    self:refresh_file_list()
  end, opts)

  vim.keymap.set("n", "t", function()
    if self.diff_state then
      self.diff_state:cycle_mode()
    end
  end, opts)

  vim.keymap.set("n", "q", function()
    self:close()
  end, opts)
end

--- Return the file entry under the cursor in the file list window.
--- Returns nil when the cursor is on a non-file line (header, directory, blank).
---@return kenjutu.FileEntry|nil
function ReviewState:selected_file()
  if not vim.api.nvim_win_is_valid(self.file_list_winnr) then
    return nil
  end
  local line = vim.api.nvim_win_get_cursor(self.file_list_winnr)[1]
  return self.line_map[line]
end

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
---@param on_close function callback to run after review screen is closed
---@return kenjutu.ReviewState
function M.open(dir, commit, log_bufnr, on_close)
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

  local diff_state = diff.create(diff_anchor_winnr, dir, commit.change_id, commit.commit_id)

  local s = ReviewState.new({
    dir = dir,
    change_id = commit.change_id,
    commit_id = commit.commit_id,
    file_list_bufnr = file_list_bufnr,
    file_list_winnr = file_list_winnr,
    log_bufnr = log_bufnr,
    on_close = on_close,
    diff_state = diff_state,
  })

  diff_state:set_callbacks({
    focus_file_list = function()
      if vim.api.nvim_win_is_valid(s.file_list_winnr) then
        vim.api.nvim_set_current_win(s.file_list_winnr)
      end
    end,
    move_selection = function(direction)
      s:move_selection(direction)
    end,
    close = function()
      s:close()
    end,
    on_mark = function()
      s:refresh_file_list()
    end,
    navigate_to = function(file_path, line, side)
      s:navigate_to(file_path, line, side)
    end,
  })

  -- Restore focus to file list after diff layout creation
  vim.api.nvim_set_current_win(file_list_winnr)
  vim.api.nvim_win_set_width(file_list_winnr, 40)

  s:setup_file_list_keymaps()

  local prev_file_path = nil
  vim.api.nvim_create_autocmd("CursorMoved", {
    buffer = file_list_bufnr,
    callback = function()
      if not vim.api.nvim_win_is_valid(file_list_winnr) then
        return
      end
      local file = s:selected_file()
      if not file then
        return
      end
      local path = utils.file_path(file)
      if path ~= prev_file_path then
        prev_file_path = path
        s:update_diff_view()
      end
    end,
  })

  local tab_page = vim.api.nvim_tabpage_get_number(0)
  vim.api.nvim_create_autocmd("TabClosed", {
    once = true,
    callback = function(e)
      local closed_tab = tonumber(e.match)
      if closed_tab == tab_page then
        s.diff_state:cleanup()
      end
    end,
  })

  kjn.files(dir, commit.change_id, function(err, result)
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
    s.line_map = file_list.render(s.file_list_bufnr, s.files, s.file_list_winnr)
  end)

  return s
end

return M
