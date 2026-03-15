local utils = require("kenjutu.utils")
local mod_comments = require("kenjutu.comments")

local M = {}

--- Create a scratch buffer for use in a diff pane.
---@param wipe? boolean
---@return integer bufnr
local function create_scratch_buf(wipe)
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = wipe and "wipe" or "hide"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].modifiable = false
  return bufnr
end

---@param winnr integer
local function enable_diff(winnr)
  vim.api.nvim_win_call(winnr, function()
    vim.cmd("diffthis")
  end)
  vim.wo[winnr].number = true
  vim.wo[winnr].relativenumber = false
  vim.wo[winnr].signcolumn = "auto"
  vim.wo[winnr].wrap = false
  vim.wo[winnr].foldenable = true
  vim.wo[winnr].foldmethod = "diff"
  vim.wo[winnr].foldlevel = 0
  vim.wo[winnr].cursorline = true
end

---@param anchor_winnr integer
---@return integer
local function create_layout(anchor_winnr)
  local left_bufnr = create_scratch_buf(true)
  local right_bufnr = create_scratch_buf(true)

  vim.api.nvim_set_current_win(anchor_winnr)
  vim.api.nvim_win_set_buf(anchor_winnr, left_bufnr)
  vim.cmd("rightbelow vsplit")
  local right_winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(right_winnr, right_bufnr)

  return right_winnr
end

---@param mode "remaining" | "reviewed"
---@param side "left" | "right"
---@return "base" | "marker" | "target"
local function tree_for_side(mode, side)
  if mode == "remaining" then
    return side == "left" and "marker" or "target"
  else
    return side == "left" and "base" or "marker"
  end
end

---@class kenjutu.DiffState
---@field left_winnr integer inherited from parent. Should not be closed
---@field right_winnr integer
---@field mode "remaining" | "reviewed"
---@field file_path string|nil
---@field change_id string
---@field keymap_installer fun(bufnr: integer)|nil
---@field created_buffers integer[]
local DiffState = {}
DiffState.__index = DiffState

---@class kenjutu.DiffStateInitOpts
---@field anchor_winnr integer
---@field change_id string

--- @param opts kenjutu.DiffStateInitOpts
--- @return kenjutu.DiffState
function DiffState:new(opts)
  local right_winnr = create_layout(opts.anchor_winnr)

  --- @type kenjutu.DiffState
  local obj = {
    left_winnr = opts.anchor_winnr,
    right_winnr = right_winnr,
    change_id = opts.change_id,
    mode = "remaining",
    pane = nil,
    file_path = nil,
    keymap_installer = nil,
    created_buffers = {},
  }
  setmetatable(obj, self)
  return obj
end

---@param winnr integer
local function diff_off_win(winnr)
  if vim.api.nvim_win_is_valid(winnr) then
    vim.api.nvim_win_call(winnr, function()
      vim.cmd("diffoff")
    end)
  end
end

---@class kenjutu.DiffState.SideInfo
---@field side "left"|"right"
---@field tree "base"|"marker"|"target"

function DiffState:current_side()
  local winnr = vim.api.nvim_get_current_win()
  local side = winnr == self.left_winnr and "left" or winnr == self.right_winnr and "right" or nil
  if not side then
    return nil
  end

  ---@type kenjutu.DiffState.SideInfo
  return {
    side = side,
    tree = tree_for_side(self.mode, side),
  }
end

---@param side "left"|"right"
---@param content string
function DiffState:setup_diff_win(side, content)
  local ft = self.file_path and vim.filetype.match({ filename = self.file_path }) or nil
  local winnr = side == "left" and self.left_winnr or self.right_winnr
  local tree = tree_for_side(self.mode, side)

  ---@param bufnr integer
  local function setup_window(bufnr)
    vim.api.nvim_win_set_buf(winnr, bufnr)
    enable_diff(winnr)
    if self.keymap_installer then
      self.keymap_installer(bufnr)
    end
    if ft then
      vim.bo[bufnr].filetype = ft
    end
    vim.bo[bufnr].modifiable = true
    local lines = vim.split(content or "", "\n", { plain = true })
    if #lines > 0 and lines[#lines] == "" then
      table.remove(lines)
    end
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
    vim.bo[bufnr].modifiable = false
  end

  local buf_name = "kenjutu://" .. self.change_id .. "/" .. self.file_path .. ":" .. tree
  local existing_bufnr = vim.fn.bufnr(buf_name)
  if existing_bufnr ~= -1 then
    setup_window(existing_bufnr)
    return
  end

  local bufnr = create_scratch_buf()
  vim.api.nvim_buf_set_name(bufnr, buf_name)
  table.insert(self.created_buffers, bufnr)
  setup_window(bufnr)
end

---@param side "left"|"right"
---@return integer|nil
function DiffState:buf(side)
  local winnr = side == "left" and self.left_winnr or self.right_winnr
  return vim.api.nvim_win_get_buf(winnr)
end

---@param setup_keymaps fun(bufnr: integer)
function DiffState:set_keymaps(setup_keymaps)
  self.keymap_installer = setup_keymaps
  local left_bufnr = self:buf("left")
  local right_bufnr = self:buf("right")
  if left_bufnr then
    setup_keymaps(left_bufnr)
  end
  if right_bufnr then
    setup_keymaps(right_bufnr)
  end
end

---@param file kenjutu.FileEntry
---@param loader fun(tree_kind: kenjutu.TreeKind, cb: fun(err: string|nil, content: string|nil))
---@param comments kenjutu.PortedComment[]
function DiffState:set_file(file, loader, comments)
  self.file_path = utils.file_path(file)
  self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"

  utils.await_all({
    left = function(cb)
      loader(tree_for_side(self.mode, "left"), cb)
    end,
    right = function(cb)
      loader(tree_for_side(self.mode, "right"), cb)
    end,
  }, function(err, results)
    if err then
      vim.notify("kjn blob: " .. err, vim.log.levels.ERROR)
      return
    end

    diff_off_win(self.left_winnr)
    diff_off_win(self.right_winnr)

    self:setup_diff_win("left", results and results.left)
    self:setup_diff_win("right", results and results.right)
    self:update_signs(comments)
  end)
end

---@param loader fun(tree_kind: kenjutu.TreeKind, cb: fun(err: string|nil, content: string|nil))
---@param comments kenjutu.PortedComment[]
function DiffState:toggle_mode(loader, comments)
  -- M T remaining
  --  ↕
  -- B M reviewed
  local update_opts = self.mode == "remaining"
      and {
        new_tree = "base",
        swap_from_side = "left",
        new_tree_winnr = self.left_winnr,
        swap_to_winnr = self.right_winnr,
      }
    or {
      new_tree = "target",
      swap_from_side = "right",
      new_tree_winnr = self.right_winnr,
      swap_to_winnr = self.left_winnr,
    }

  loader(update_opts.new_tree, function(err, content)
    if err then
      vim.notify("kjn blob: " .. err, vim.log.levels.ERROR)
      return
    end
    self.mode = self.mode == "remaining" and "reviewed" or "remaining"
    local keep_bufnr = self:buf(update_opts.swap_from_side)
    if keep_bufnr == nil then
      vim.notify("Failed to get buffer for toggling mode", vim.log.levels.ERROR)
      return
    end
    diff_off_win(update_opts.new_tree_winnr)
    diff_off_win(update_opts.swap_to_winnr)

    vim.api.nvim_win_set_buf(update_opts.swap_to_winnr, keep_bufnr)
    enable_diff(update_opts.swap_to_winnr)

    self:setup_diff_win(update_opts.swap_from_side, content or "")
    self:update_signs(comments)
  end)
end

---@param is_visual boolean
---@param on_mark fun(content: string) callback with the new content of the marker buffer after the change is applied
function DiffState:mark_action(is_visual, on_mark)
  local bufnr = vim.api.nvim_get_current_buf()
  local left_bufnr = self:buf("left")
  local right_bufnr = self:buf("right")
  if not left_bufnr or not right_bufnr then
    return
  end

  local side = bufnr == left_bufnr and "left" or bufnr == right_bufnr and "right" or nil
  assert(side, "current buffer is not part of the diff panes")

  local marker_bufnr = self.mode == "remaining" and left_bufnr or right_bufnr
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
  local content_str = table.concat(maker_contents, "\n") .. "\n"

  on_mark(content_str)
end

---@param commit_id string
---@param dir string
---@param on_create fun()
function DiffState:new_comment(dir, commit_id, on_create)
  local file_path = self.file_path
  if not file_path then
    return
  end
  local side_info = self:current_side()
  if not side_info then
    return
  end
  local tree = side_info.tree
  if tree == "marker" then
    vim.notify("Cannot comment on the marker version of the file", vim.log.levels.WARN)
    return
  end

  mod_comments.open_new_comment({
    change_id = self.change_id,
    file_path = file_path,
    commit_id = commit_id,
    dir = dir,
    side = tree == "base" and "Old" or "New",
    on_create = on_create,
  })
end

---@param comments kenjutu.PortedComment[]
function DiffState:open_thread_at_cursor(comments)
  local file_path = self.file_path
  if not file_path then
    return
  end
  local side_info = self:current_side()
  if not side_info then
    return
  end
  if side_info.tree == "marker" then
    return
  end

  local side = side_info.tree == "base" and "Old" or "New"
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local at_line = mod_comments.comments_at_line(comments, cursor_line, side)
  if #at_line == 0 then
    return
  end

  mod_comments.open_thread({
    file_path = file_path,
    line = cursor_line,
    side = side,
    comments = at_line,
  })
end

---@param comments kenjutu.PortedComment[]
---@param dir string
---@param on_resolve fun()
function DiffState:open_comment_list(comments, dir, on_resolve)
  local file_path = self.file_path
  if not file_path then
    return
  end
  mod_comments.open_comment_list({
    file_path = file_path,
    comments = comments,
    dir = dir,
    change_id = self.change_id,
    on_resolve = on_resolve,
    on_select = function(pc)
      if not pc.ported_line then
        return
      end
      local side = pc.comment.side
      local winnr
      if self.mode == "remaining" and side == "New" then
        winnr = self.right_winnr
      elseif self.mode == "reviewed" and side == "Old" then
        winnr = self.left_winnr
      end
      if winnr and vim.api.nvim_win_is_valid(winnr) then
        vim.api.nvim_set_current_win(winnr)
        vim.api.nvim_win_set_cursor(winnr, { pc.ported_line, 0 })
      end
    end,
  })
end

---@param comments kenjutu.PortedComment[]
function DiffState:update_signs(comments)
  if self.mode == "remaining" then
    local right_bufnr = self:buf("right")
    if right_bufnr then
      mod_comments.place_signs(right_bufnr, comments, "New")
    end
  else
    local left_bufnr = self:buf("left")
    if left_bufnr then
      mod_comments.place_signs(left_bufnr, comments, "Old")
    end
  end
end

function DiffState:next_comment()
  mod_comments.goto_next_comment()
end

function DiffState:prev_comment()
  mod_comments.goto_prev_comment()
end

function DiffState:close()
  diff_off_win(self.left_winnr)
  diff_off_win(self.right_winnr)

  if vim.api.nvim_win_is_valid(self.right_winnr) then
    vim.api.nvim_win_close(self.right_winnr, true)
  end
  self:cleanup()
end

function DiffState:cleanup()
  for _, bufnr in ipairs(self.created_buffers) do
    if vim.api.nvim_buf_is_valid(bufnr) then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
  end
end

---@param anchor_winnr integer
---@param change_id string
---@return kenjutu.DiffState
function M.create(anchor_winnr, change_id)
  local state = DiffState:new({
    anchor_winnr = anchor_winnr,
    change_id = change_id,
  })
  return state
end

return M
