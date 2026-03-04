local kjn = require("kenjutu.kjn")

local M = {}

---@alias TreeKind  "base" | "marker" | "target"

---@class kenjutu.DiffPane
---@field left_bufnr integer
---@field right_bufnr integer
---@field left_winnr integer
---@field right_winnr integer

---@class kenjutu.DiffState
---@field anchor_winnr integer  the parent window (not created by us, must not be closed)
---@field pane kenjutu.DiffPane|nil
---@field mode "remaining" | "reviewed"
---@field created_winnrs integer[]  windows created by create_layout() that should be closed on cleanup
---@field file_path string|nil
---@field base_blob string|nil
---@field marker_blob string|nil
---@field target_blob string|nil
local DiffState = {}
DiffState.__index = DiffState

--- @param anchor_winnr integer
--- @return kenjutu.DiffState
function DiffState:new(anchor_winnr)
  --- @type kenjutu.DiffState
  local obj = {
    anchor_winnr = anchor_winnr,
    mode = "remaining",
    pane = nil,
    created_winnrs = {},
    file_path = nil,
    base_blob = nil,
    marker_blob = nil,
    target_blob = nil,
  }
  setmetatable(obj, self)
  return obj
end

--- Create a scratch buffer for use in a diff pane.
---@return integer bufnr
local function create_scratch_buf()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].modifiable = false
  return bufnr
end

---@param winnr integer
local function setup_diff_win(winnr)
  vim.api.nvim_win_call(winnr, function()
    vim.cmd("diffthis")
  end)
  vim.wo[winnr].number = true
  vim.wo[winnr].relativenumber = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false
  vim.wo[winnr].foldenable = true
  vim.wo[winnr].foldmethod = "diff"
  vim.wo[winnr].foldlevel = 0
  vim.wo[winnr].cursorline = true
end

---@param winnr integer
local function diff_off_win(winnr)
  if vim.api.nvim_win_is_valid(winnr) then
    vim.api.nvim_win_call(winnr, function()
      vim.cmd("diffoff")
    end)
  end
end

---@param change_id string
---@param commit_id string
---@param file_path string
---@param old_path string|nil
---@param tree_kind TreeKind
---@return string[]
local function blob_args(change_id, commit_id, file_path, old_path, tree_kind)
  local args = {
    "blob",
    "--change-id",
    change_id,
    "--commit",
    commit_id,
    "--file",
    file_path,
    "--tree",
    tree_kind,
  }
  if old_path and old_path ~= file_path then
    table.insert(args, "--old-path")
    table.insert(args, old_path)
  end
  return args
end

---@param dir string
---@param change_id string
---@param commit_id string
---@param file_path string
---@param old_path string|nil
---@param tree_kind TreeKind
---@param callback fun(err: string|nil, content: string)
local function fetch_blob(dir, change_id, commit_id, file_path, old_path, tree_kind, callback)
  local args = blob_args(change_id, commit_id, file_path, old_path, tree_kind)
  kjn.run_raw(dir, args, function(err, stdout)
    if err then
      callback(err, "")
      return
    end
    callback(nil, stdout or "")
  end)
end

--- Collect N async results, then call on_done once all arrive.
---@param n integer number of expected results
---@param on_done fun(results: table<string, string>) map of key -> content
---@return fun(key: string, err: string|nil, content: string) collector
local function async_collect(n, on_done)
  local results = {}
  local count = 0
  local failed = false
  return function(key, err, content)
    if failed then
      return
    end
    if err then
      failed = true
      vim.notify("kjn blob (" .. key .. "): " .. err, vim.log.levels.ERROR)
      return
    end
    results[key] = content
    count = count + 1
    if count == n then
      on_done(results)
    end
  end
end

--- Create the split layout with empty placeholder buffers.
--- Called once at creation time. Windows and buffers persist for the
--- lifetime of the DiffState.
---@param setup_keymaps fun(bufnr: integer)
function DiffState:create_layout(setup_keymaps)
  local left_bufnr = create_scratch_buf()
  local right_bufnr = create_scratch_buf()

  vim.api.nvim_set_current_win(self.anchor_winnr)
  vim.api.nvim_win_set_buf(self.anchor_winnr, left_bufnr)
  vim.cmd("rightbelow vsplit")
  local right_winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(right_winnr, right_bufnr)

  setup_diff_win(self.anchor_winnr)
  setup_diff_win(right_winnr)

  self.pane = {
    left_bufnr = left_bufnr,
    right_bufnr = right_bufnr,
    left_winnr = self.anchor_winnr,
    right_winnr = right_winnr,
  }
  table.insert(self.created_winnrs, right_winnr)

  setup_keymaps(left_bufnr)
  setup_keymaps(right_bufnr)
end

--- Split content string into lines, removing trailing empty line from final newline.
---@param content string|nil
---@return string[]
local function split_lines(content)
  local lines = vim.split(content or "", "\n", { plain = true })
  if #lines > 1 and lines[#lines] == "" then
    table.remove(lines)
  end
  return lines
end

--- Update the buffer contents and filetype to reflect current blobs and mode.
function DiffState:update_wins()
  local pane = self.pane
  if not pane then
    return
  end

  local left_content = self.mode == "remaining" and self.marker_blob or self.base_blob
  local right_content = self.mode == "remaining" and self.target_blob or self.marker_blob
  local ft = self.file_path and vim.filetype.match({ filename = self.file_path }) or nil

  if vim.api.nvim_buf_is_valid(pane.left_bufnr) then
    vim.bo[pane.left_bufnr].modifiable = true
    vim.api.nvim_buf_set_lines(pane.left_bufnr, 0, -1, false, split_lines(left_content))
    vim.bo[pane.left_bufnr].modifiable = false
    vim.bo[pane.left_bufnr].filetype = ft or ""
  end
  if vim.api.nvim_buf_is_valid(pane.right_bufnr) then
    vim.bo[pane.right_bufnr].modifiable = true
    vim.api.nvim_buf_set_lines(pane.right_bufnr, 0, -1, false, split_lines(right_content))
    vim.bo[pane.right_bufnr].modifiable = false
    vim.bo[pane.right_bufnr].filetype = ft or ""
  end
end

--- Load a new file into the diff view. Fetches blobs asynchronously and
--- updates the existing buffers in-place when all arrive.
---@param file kenjutu.FileEntry
---@param dir string
---@param change_id string
---@param commit_id string
function DiffState:set_file(file, dir, change_id, commit_id)
  local file_path = file.newPath or file.oldPath or ""
  self.file_path = file_path
  self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"

  local collect = async_collect(3, function(blobs)
    if not vim.api.nvim_win_is_valid(self.anchor_winnr) then
      return
    end
    self.base_blob = blobs.base
    self.marker_blob = blobs.marker
    self.target_blob = blobs.target
    self:update_wins()
  end)

  fetch_blob(dir, change_id, commit_id, file_path, file.oldPath, "base", function(err, content)
    collect("base", err, content)
  end)
  fetch_blob(dir, change_id, commit_id, file_path, file.oldPath, "marker", function(err, content)
    collect("marker", err, content)
  end)
  fetch_blob(dir, change_id, commit_id, file_path, file.oldPath, "target", function(err, content)
    collect("target", err, content)
  end)
end

---@return {old_start: integer, old_lines: integer, new_start: integer, new_lines: integer}[]
function DiffState:compute_hunks()
  local left_content = self.mode == "remaining" and self.marker_blob or self.base_blob
  local right_content = self.mode == "remaining" and self.target_blob or self.marker_blob
  if not left_content or not right_content then
    return {}
  end
  ---@type integer[][]
  ---@diagnostic disable-next-line: assign-type-mismatch result_type: "indices" returns array of [old_start, old_lines, new_start, new_lines]
  local raw = vim.diff(left_content, right_content, { result_type = "indices" })
  local hunks = {}
  for _, h in ipairs(raw) do
    table.insert(hunks, {
      old_start = h[1],
      old_lines = h[2],
      new_start = h[3],
      new_lines = h[4],
    })
  end
  return hunks
end

---@param cursor_line integer 1-indexed buffer line number
---@param side "old"|"new"
---@return {old_start: integer, old_lines: integer, new_start: integer, new_lines: integer}|nil
function DiffState:hunk_at(cursor_line, side)
  local hunks = self:compute_hunks()
  for _, h in ipairs(hunks) do
    local start = side == "old" and h.old_start or h.new_start
    local count = side == "old" and h.old_lines or h.new_lines
    if count > 0 and cursor_line >= start and cursor_line < start + count then
      return h
    end
  end
  return nil
end

---@param bufnr integer
---@return "old"|"new"|nil
function DiffState:which_side(bufnr)
  if not self.pane then
    return nil
  end
  if bufnr == self.pane.left_bufnr then
    return "old"
  elseif bufnr == self.pane.right_bufnr then
    return "new"
  end
  return nil
end

---@alias DiffAction "mark" | "unmark"

---@param bufnr integer
---@param cursor_line integer 1-indexed
---@return { hunk: {old_start: integer, old_lines: integer, new_start: integer, new_lines: integer}, action: DiffAction }|nil
function DiffState:resolve_hunk(bufnr, cursor_line)
  local side = self:which_side(bufnr)
  if not side then
    return nil
  end
  local hunk = self:hunk_at(cursor_line, side)
  if not hunk then
    return nil
  end
  local action = self.mode == "remaining" and "mark" or "unmark"
  return { hunk = hunk, action = action }
end

function DiffState:close()
  if not self then
    return
  end

  if self.pane then
    diff_off_win(self.pane.left_winnr)
    diff_off_win(self.pane.right_winnr)
  end

  -- Close only the windows we created (NOT the anchor)
  for _, winnr in ipairs(self.created_winnrs or {}) do
    if vim.api.nvim_win_is_valid(winnr) then
      vim.api.nvim_win_close(winnr, true)
    end
  end
end

---@param opts { anchor_winnr: integer, setup_keymaps: fun(bufnr: integer) }
---@return kenjutu.DiffState
function M.create(opts)
  local state = DiffState:new(opts.anchor_winnr)
  state:create_layout(opts.setup_keymaps)
  return state
end

return M
