local utils = require("kenjutu.utils")
local mod_comments = require("kenjutu.comments")

local M = {}

---@class kenjutu.DiffPane
---@field left_winnr integer
---@field right_winnr integer

---@class kenjutu.DiffState
---@field anchor_winnr integer  the parent window (not created by us, must not be closed)
---@field pane kenjutu.DiffPane|nil
---@field mode "remaining" | "reviewed"
---@field created_winnrs integer[]  windows created by create_layout() that should be closed on cleanup
---@field file_path string|nil
---@field change_id string
---@field keymap_installer fun(bufnr: integer)|nil
local DiffState = {}
DiffState.__index = DiffState

---@class kenjutu.DiffStateInitOpts
---@field anchor_winnr integer
---@field change_id string

--- @param opts kenjutu.DiffStateInitOpts
--- @return kenjutu.DiffState
function DiffState:new(opts)
  --- @type kenjutu.DiffState
  local obj = {
    anchor_winnr = opts.anchor_winnr,
    change_id = opts.change_id,
    mode = "remaining",
    pane = nil,
    created_winnrs = {},
    file_path = nil,
    keymap_installer = nil,
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
  vim.wo[winnr].signcolumn = "auto"
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

---@class kenjutu.DiffState.SideInfo
---@field side "left"|"right"
---@field tree "base"|"marker"|"target"

function DiffState:current_side()
  local pane = self.pane
  if not pane then
    return nil
  end
  local winnr = vim.api.nvim_get_current_win()
  local side = winnr == pane.left_winnr and "left" or winnr == pane.right_winnr and "right" or nil
  if not side then
    return nil
  end
  local tree = self.mode == "remaining" and (side == "left" and "marker" or "target")
    or (side == "left" and "base" or "marker")

  ---@type kenjutu.DiffState.SideInfo
  return {
    side = side,
    tree = tree,
  }
end

--- Create the split layout with empty placeholder buffers.
--- Called once at creation time. Windows and buffers persist for the
--- lifetime of the DiffState.
function DiffState:create_layout()
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
    left_winnr = self.anchor_winnr,
    right_winnr = right_winnr,
  }
  table.insert(self.created_winnrs, right_winnr)
end

--- Create a scratch buffer, place it in the given window, and set up diff.
--- The old buffer is auto-wiped (bufhidden=wipe) when displaced.
---@param winnr integer
---@param tree "base"|"marker"|"target"
---@param ft string|nil
function DiffState:place_scratch_buf(winnr, tree, ft)
  local bufnr = create_scratch_buf()
  if ft then
    vim.bo[bufnr].filetype = ft
  end
  vim.api.nvim_win_set_buf(winnr, bufnr)
  if self.file_path then
    local buf_name = "kenjutu://" .. self.change_id .. "/" .. self.file_path .. ":" .. tree
    vim.api.nvim_buf_set_name(bufnr, buf_name)
  end
  setup_diff_win(winnr)
  if self.keymap_installer then
    self.keymap_installer(bufnr)
  end
end

---@param left_tree "base"|"marker"|"target"
---@param right_tree "base"|"marker"|"target"
---@param ft string|nil
function DiffState:replace_pane_buffers(left_tree, right_tree, ft)
  local pane = self.pane
  if not pane then
    return
  end
  self:place_scratch_buf(pane.left_winnr, left_tree, ft)
  self:place_scratch_buf(pane.right_winnr, right_tree, ft)
end

---@param side "left"|"right"
---@return integer|nil
function DiffState:buf(side)
  local pane = self.pane
  if not pane then
    return nil
  end
  local winnr = side == "left" and pane.left_winnr or pane.right_winnr
  return vim.api.nvim_win_get_buf(winnr)
end

---@param setup_keymaps fun(bufnr: integer)
function DiffState:set_keymaps(setup_keymaps)
  self.keymap_installer = setup_keymaps
  if not self.pane then
    return
  end
  local left_bufnr = self:buf("left")
  local right_bufnr = self:buf("right")
  if left_bufnr then
    setup_keymaps(left_bufnr)
  end
  if right_bufnr then
    setup_keymaps(right_bufnr)
  end
end

---@param side "left"|"right"
---@param content string
---@param ft string|nil
function DiffState:set_buf_contents(side, content, ft)
  local bufnr = self:buf(side)
  if not bufnr then
    return
  end
  vim.bo[bufnr].modifiable = true
  local lines = vim.split(content or "", "\n", { plain = true })
  if #lines > 0 and lines[#lines] == "" then
    table.remove(lines)
  end
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false
  if ft then
    vim.bo[bufnr].filetype = ft
  end
end

--- Load a new file into the diff view. Fetches both blobs in parallel,
--- then atomically replaces the pane buffers once both arrive.
---@param file kenjutu.FileEntry
---@param loader fun(tree_kind: kenjutu.TreeKind, cb: fun(err: string|nil, content: string|nil))
---@param comments kenjutu.PortedComment[]
function DiffState:set_file(file, loader, comments)
  self.file_path = utils.file_path(file)
  self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"

  local ft = self.file_path and vim.filetype.match({ filename = self.file_path }) or nil
  local side_opts = self.mode == "remaining" and { left = "marker", right = "target" }
    or { left = "base", right = "marker" }

  utils.await_all({
    left = function(cb)
      loader(side_opts.left, cb)
    end,
    right = function(cb)
      loader(side_opts.right, cb)
    end,
  }, function(err, results)
    if err or results == nil then
      vim.notify("kjn blob: " .. err, vim.log.levels.ERROR)
      return
    end
    self:replace_pane_buffers(side_opts.left, side_opts.right, ft)
    self:set_buf_contents("left", results.left or "", ft)
    self:set_buf_contents("right", results.right or "", ft)
    self:update_signs(comments)
  end)
end

---@param loader fun(tree_kind: kenjutu.TreeKind, cb: fun(err: string|nil, content: string|nil))
---@param comments kenjutu.PortedComment[]
function DiffState:toggle_mode(loader, comments)
  local pane = self.pane
  if not pane then
    return
  end
  -- M T remaining
  --  ↕
  -- B M reviewed
  local update_opts = self.mode == "remaining"
      and {
        new_tree = "base",
        swap_from_side = "left",
        new_tree_winnr = pane.left_winnr,
        swap_to_winnr = pane.right_winnr,
        new_tree_side = "left",
      }
    or {
      new_tree = "target",
      swap_from_side = "right",
      new_tree_winnr = pane.right_winnr,
      swap_to_winnr = pane.left_winnr,
      new_tree_side = "right",
    }

  loader(update_opts.new_tree, function(err, content)
    if err then
      vim.notify("kjn blob: " .. err, vim.log.levels.ERROR)
      return
    end
    local ft = self.file_path and vim.filetype.match({ filename = self.file_path }) or nil
    self.mode = self.mode == "remaining" and "reviewed" or "remaining"
    local keep_bufnr = self:buf(update_opts.swap_from_side)
    if keep_bufnr == nil then
      vim.notify("Failed to get buffer for toggling mode", vim.log.levels.ERROR)
      return
    end
    vim.api.nvim_win_set_buf(update_opts.swap_to_winnr, keep_bufnr)
    self:place_scratch_buf(update_opts.new_tree_winnr, update_opts.new_tree, ft)
    setup_diff_win(update_opts.swap_to_winnr)
    self:set_buf_contents(update_opts.new_tree_side, content or "", ft)
    self:update_signs(comments)
  end)
end

--- Mark the current change as "remaining" or "reviewed" by performing a diffput/diffget.
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
function DiffState:open_comment_list(comments)
  local file_path = self.file_path
  if not file_path then
    return
  end
  local pane = self.pane
  if not pane then
    return
  end
  mod_comments.open_comment_list({
    file_path = file_path,
    comments = comments,
    on_select = function(pc)
      if not pc.ported_line then
        return
      end
      local side = pc.comment.side
      local winnr
      if self.mode == "remaining" and side == "New" then
        winnr = pane.right_winnr
      elseif self.mode == "reviewed" and side == "Old" then
        winnr = pane.left_winnr
      end
      if winnr and vim.api.nvim_win_is_valid(winnr) then
        vim.api.nvim_set_current_win(winnr)
        vim.api.nvim_win_set_cursor(winnr, { pc.ported_line, 0 })
      end
    end,
    on_open_thread = function(pc)
      if not pc.ported_line then
        return
      end
      mod_comments.open_thread({
        file_path = file_path,
        line = pc.ported_line,
        side = pc.comment.side,
        comments = mod_comments.comments_at_line(comments, pc.ported_line, pc.comment.side),
      })
    end,
  })
end

---@param comments kenjutu.PortedComment[]
function DiffState:update_signs(comments)
  if not self.pane then
    return
  end
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

---@param anchor_winnr integer
---@param change_id string
---@return kenjutu.DiffState
function M.create(anchor_winnr, change_id)
  local state = DiffState:new({
    anchor_winnr = anchor_winnr,
    change_id = change_id,
  })
  state:create_layout()
  return state
end

return M
