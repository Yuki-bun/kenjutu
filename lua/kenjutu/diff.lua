local kjn = require("kenjutu.kjn")
local utils = require("kenjutu.utils")

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
local DiffState = {}
DiffState.__index = DiffState

---@class kenjutu.DiffStateInitOpts
---@field anchor_winnr integer

--- @param opts kenjutu.DiffStateInitOpts
--- @return kenjutu.DiffState
function DiffState:new(opts)
  --- @type kenjutu.DiffState
  local obj = {
    anchor_winnr = opts.anchor_winnr,
    mode = "remaining",
    pane = nil,
    created_winnrs = {},
    file_path = nil,
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

---@param side "left"|"right"
---@param content string
---@param ft string|nil
function DiffState:set_buf_contents(side, content, ft)
  local pane = self.pane
  if not pane then
    return
  end
  local bufnr = side == "left" and pane.left_bufnr or pane.right_bufnr
  assert(vim.api.nvim_buf_is_valid(bufnr), "buffer was unexpectedly invalid")
  vim.bo[bufnr].modifiable = true
  local lines = vim.split(content or "", "\n", { plain = true })
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false
  if ft then
    vim.bo[bufnr].filetype = ft
  end
end

--- Load a new file into the diff view. Fetches blobs asynchronously and
--- updates the existing buffers in-place when all arrive.
---@param file kenjutu.FileEntry
---@param dir string
---@param change_id string
---@param commit_id string
function DiffState:set_file(file, dir, change_id, commit_id)
  self.file_path = utils.file_path(file)
  self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"
  local ft = self.file_path and vim.filetype.match({ filename = self.file_path }) or nil

  ---@param side "base"|"marker"|"target"
  ---@param pane_side "left"|"right"
  local function load_content(side, pane_side)
    fetch_blob(dir, change_id, commit_id, self.file_path, file.oldPath, side, function(err, content)
      if err then
        vim.notify("kjn blob (" .. side .. "): " .. err, vim.log.levels.ERROR)
        return
      end
      self:set_buf_contents(pane_side, content, ft)
    end)
  end

  if self.mode == "remaining" then
    load_content("marker", "left")
    load_content("target", "right")
  else
    load_content("base", "left")
    load_content("marker", "right")
  end

  self:update_buf_names()
end

function DiffState:update_buf_names()
  local pane = self.pane
  if not pane then
    return
  end

  ---@param side "base"|"marker"|"target"
  local function buf_name(side)
    return "kjn://" .. self.file_path .. ":" .. side
  end

  --- Rename the buffer and remove the duplicated buffer with the old name
  --- that nvim_buf_set_name creates.
  --- see https://github.com/neovim/neovim/issues/20349
  ---@param bufnr integer
  ---@param new_name string
  local function rename_buf(bufnr, new_name)
    local old_name = vim.api.nvim_buf_get_name(bufnr)
    if old_name == new_name then
      return
    end
    vim.api.nvim_buf_set_name(bufnr, new_name)
    local old_bufnr = vim.fn.bufnr(old_name)
    if old_name ~= "" and old_bufnr ~= bufnr and vim.api.nvim_buf_is_valid(old_bufnr) then
      vim.api.nvim_buf_delete(old_bufnr, {})
    end
  end

  local new_left_name = self.mode == "remaining" and buf_name("marker") or buf_name("base")
  local new_right_name = self.mode == "remaining" and buf_name("target") or buf_name("marker")

  -- B M (reviewed)
  -- B T     ↕
  -- M T (remaining)
  -- specific order of rename to avoid buffer name collision
  if self.mode == "remaining" then
    rename_buf(pane.right_bufnr, new_right_name)
    rename_buf(pane.left_bufnr, new_left_name)
  else
    rename_buf(pane.left_bufnr, new_left_name)
    rename_buf(pane.right_bufnr, new_right_name)
  end
end

---@return {old_start: integer, old_lines: integer, new_start: integer, new_lines: integer}[]
function DiffState:compute_hunks()
  local left_content = vim.api.nvim_buf_get_lines(self.pane.left_bufnr, 0, -1, false)
  local right_content = vim.api.nvim_buf_get_lines(self.pane.right_bufnr, 0, -1, false)
  local left_joined = table.concat(left_content, "\n")
  local right_joined = table.concat(right_content, "\n")

  ---@type integer[][]
  ---@diagnostic disable-next-line: assign-type-mismatch result_type: "indices" returns array of [old_start, old_lines, new_start, new_lines]
  local raw = vim.diff(left_joined, right_joined, { result_type = "indices" })
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

--- Mark the current change as "remaining" or "reviewed" by performing a diffput/diffget.
---@param is_visual boolean
---@param on_mark fun(content: string) callback with the new content of the marker buffer after the change is applied
function DiffState:mark_action(is_visual, on_mark)
  local bufnr = vim.api.nvim_get_current_buf()
  local pane = self.pane
  if not pane then
    return
  end

  local side = bufnr == pane.left_bufnr and "left" or bufnr == pane.right_bufnr and "right" or nil
  assert(side, "current buffer is not part of the diff panes")

  local marker_bufnr = self.mode == "remaining" and pane.left_bufnr or pane.right_bufnr
  local is_marker = bufnr == marker_bufnr
  local cmd = is_marker and "diffget" or "diffput"

  local marker_buf_opts = vim.bo[marker_bufnr]
  marker_buf_opts.modifiable = true
  if is_visual then
    local v_start = vim.fn.line("v")
    local v_end = vim.fn.line(".")
    if v_start > v_end then
      v_start, v_end = v_end, v_start
    end
    vim.cmd(string.format("%d,%d%s", v_start, v_end, cmd))
  else
    vim.cmd(cmd)
  end
  marker_buf_opts.modifiable = false

  local maker_contents = vim.api.nvim_buf_get_lines(marker_bufnr, 0, -1, false)
  local content_str = table.concat(maker_contents, "\n")

  on_mark(content_str)
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
  local state = DiffState:new({
    anchor_winnr = opts.anchor_winnr,
  })
  state:create_layout(opts.setup_keymaps)
  return state
end

return M
