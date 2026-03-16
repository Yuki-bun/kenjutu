local utils = require("kenjutu.utils")
local kjn = require("kenjutu.kjn")
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

---@alias kenjutu.DiffMode "remaining" | "reviewed" | "all"

local tree_labels = { base = "Old", marker = "Reviewed", target = "New" }

---@param mode kenjutu.DiffMode
---@param side "left" | "right"
---@return "base" | "marker" | "target"
local function tree_for_side(mode, side)
  if mode == "remaining" then
    return side == "left" and "marker" or "target"
  elseif mode == "reviewed" then
    return side == "left" and "base" or "marker"
  else
    return side == "left" and "base" or "target"
  end
end

---@param change_id string
---@param file_path string
---@param tree "base" | "marker" | "target"
---@return string
local function diff_buf_name(change_id, file_path, tree)
  return "kenjutu://" .. change_id .. "/" .. file_path .. ":" .. tree
end

---@param dir string
---@param change_id string
---@param commit_id string
---@param file_path string
---@param cb fun(comments: kenjutu.PortedComment[])
local function fetch_file_comments(dir, change_id, commit_id, file_path, cb)
  kjn.get_comments(dir, change_id, commit_id, function(err, result)
    if err then
      vim.notify("Error loading comments: " .. err, vim.log.levels.ERROR)
      cb({})
      return
    end
    for _, file_comments in ipairs(result and result.files or {}) do
      if file_comments.file_path == file_path then
        cb(file_comments.comments)
        return
      end
    end
    cb({})
  end)
end

---@class kenjutu.DiffCallbacks
---@field focus_file_list fun()
---@field move_selection fun(direction: "up"|"down")
---@field close fun()
---@field on_mark fun()
---@field navigate_to fun(file_path: string, line: integer|nil, side: "New"|"Old")

---@class kenjutu.DiffState
---@field left_winnr integer inherited from parent. Should not be closed
---@field right_winnr integer
---@field mode kenjutu.DiffMode
---@field file kenjutu.FileEntry |nil
---@field dir string
---@field change_id string
---@field commit_id string
---@field callbacks kenjutu.DiffCallbacks|nil
---@field created_buffers integer[]
local DiffState = {}
DiffState.__index = DiffState

---@class kenjutu.DiffStateInitOpts
---@field anchor_winnr integer
---@field dir string
---@field change_id string
---@field commit_id string

--- @param opts kenjutu.DiffStateInitOpts
--- @return kenjutu.DiffState
function DiffState:new(opts)
  local right_winnr = create_layout(opts.anchor_winnr)

  --- @type kenjutu.DiffState
  local obj = {
    left_winnr = opts.anchor_winnr,
    right_winnr = right_winnr,
    dir = opts.dir,
    change_id = opts.change_id,
    commit_id = opts.commit_id,
    mode = "remaining",
    pane = nil,
    file = nil,
    callbacks = nil,
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
---@return integer|nil
function DiffState:buf(side)
  local winnr = side == "left" and self.left_winnr or self.right_winnr
  return vim.api.nvim_win_get_buf(winnr)
end

---@param callbacks kenjutu.DiffCallbacks
function DiffState:set_callbacks(callbacks)
  self.callbacks = callbacks
  local left_bufnr = self:buf("left")
  local right_bufnr = self:buf("right")
  if left_bufnr then
    self:install_keymaps(left_bufnr)
  end
  if right_bufnr then
    self:install_keymaps(right_bufnr)
  end
end

---@param bufnr integer
function DiffState:install_keymaps(bufnr)
  local opts = { buffer = bufnr, silent = true }
  local cb = self.callbacks
  if not cb then
    vim.notify("buffer has been created before callbacks are set", vim.log.levels.WARN)
    return
  end

  vim.keymap.set("n", "<Tab>", function()
    cb.focus_file_list()
  end, opts)

  vim.keymap.set("n", "s", function()
    self:mark_action(false)
  end, opts)
  vim.keymap.set("v", "s", function()
    self:mark_action(true)
    vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "n", false)
  end, opts)

  vim.keymap.set("n", "gj", function()
    cb.move_selection("down")
  end, opts)

  vim.keymap.set("n", "gk", function()
    cb.move_selection("up")
  end, opts)

  vim.keymap.set("n", "t", function()
    self:cycle_mode()
  end, opts)

  vim.keymap.set({ "n", "v" }, "gc", function()
    self:new_comment()
  end, opts)

  vim.keymap.set("n", "go", function()
    self:open_thread_at_cursor()
  end, opts)

  vim.keymap.set("n", "gC", function()
    self:open_comment_list()
  end, opts)

  vim.keymap.set("n", "gA", function()
    self:open_all_comments()
  end, opts)

  vim.keymap.set("n", "[x", function()
    self:prev_comment()
  end, opts)

  vim.keymap.set("n", "]x", function()
    self:next_comment()
  end, opts)

  vim.keymap.set("n", "q", function()
    cb.close()
  end, opts)
end

---@class kenjutu.SetFileOpts
---@field line integer|nil
---@field side "Old"|"New"|nil

---@param file kenjutu.FileEntry
---@param jump_opts kenjutu.SetFileOpts|nil
function DiffState:set_file(file, jump_opts)
  self.file = file
  self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"
  self:update_wins(false, jump_opts)
end

function DiffState:cycle_mode()
  local next_mode = {
    all = "remaining",
    remaining = "reviewed",
    reviewed = "all",
  }
  self.mode = next_mode[self.mode]
  self:update_wins(false)
end

---@param ignore_cache boolean
---@param jump_opts kenjutu.SetFileOpts|nil
function DiffState:update_wins(ignore_cache, jump_opts)
  local file = self.file
  if not file then
    return
  end
  local ft = vim.filetype.match({ filename = utils.file_path(file) })

  ---@param tree "base"|"marker"|"target"
  ---@param on_loaded fun(err: any, bufnr: integer)
  local function setup_buffer(tree, on_loaded)
    ---@return integer bufnr
    ---@return  boolean was_cached
    local function get_or_create_buffer()
      local buf_name = diff_buf_name(self.change_id, utils.file_path(file), tree)
      local existing_bufnr = vim.fn.bufnr(buf_name)
      if existing_bufnr ~= -1 then
        return existing_bufnr, true
      end
      local new_bufnr = create_scratch_buf()
      table.insert(self.created_buffers, new_bufnr)
      vim.api.nvim_buf_set_name(new_bufnr, buf_name)
      if ft then
        vim.bo[new_bufnr].filetype = ft
      end
      self:install_keymaps(new_bufnr)
      return new_bufnr, false
    end

    local bufnr, cached = get_or_create_buffer()
    if cached and not ignore_cache then
      on_loaded(nil, bufnr)
      return
    end

    if file.isBinary then
      vim.bo[bufnr].modifiable = true
      vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "[Binary file]" })
      vim.bo[bufnr].modifiable = false
      on_loaded(nil, bufnr)
      return
    end

    kjn.fetch_blob({
      dir = self.dir,
      change_id = self.change_id,
      commit_id = self.commit_id,
      file_path = utils.file_path(file),
      old_path = file.status == "renamed" and file.oldPath or nil,
      tree_kind = tree,
    }, function(err, content)
      if err then
        on_loaded(err, -1)
        return
      end

      local lines = vim.split(content or "", "\n", { plain = true })
      if #lines > 0 and lines[#lines] == "" then
        table.remove(lines)
      end
      vim.bo[bufnr].modifiable = true
      vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
      vim.bo[bufnr].modifiable = false

      on_loaded(nil, bufnr)
    end)
  end

  local left_tree = tree_for_side(self.mode, "left")
  local right_tree = tree_for_side(self.mode, "right")

  utils.await_all({
    left = function(cb)
      setup_buffer(left_tree, cb)
    end,
    right = function(cb)
      setup_buffer(right_tree, cb)
    end,
  }, function(err, results)
    if err then
      vim.notify("Error loading buffers: " .. err, vim.log.levels.ERROR)
      return
    end
    if not results then
      vim.notify("Unexpected error: missing results", vim.log.levels.ERROR)
      return
    end
    ---@type integer
    local left_bufnr = results.left
    ---@type integer
    local right_bufnr = results.right

    diff_off_win(self.left_winnr)
    diff_off_win(self.right_winnr)

    vim.api.nvim_win_set_buf(self.left_winnr, left_bufnr)
    vim.api.nvim_win_set_buf(self.right_winnr, right_bufnr)

    enable_diff(self.left_winnr)
    enable_diff(self.right_winnr)

    vim.wo[self.left_winnr].winbar = tree_labels[left_tree]
    vim.wo[self.right_winnr].winbar = tree_labels[right_tree]

    if jump_opts and jump_opts.line then
      local winnr
      if jump_opts.side == "New" then
        winnr = self.right_winnr
      elseif jump_opts.side == "Old" then
        winnr = self.left_winnr
      end
      if winnr and vim.api.nvim_win_is_valid(winnr) then
        vim.api.nvim_set_current_win(winnr)
        local line_count = vim.api.nvim_buf_line_count(vim.api.nvim_win_get_buf(winnr))
        local target_line = math.min(jump_opts.line, line_count)
        vim.api.nvim_win_set_cursor(winnr, { target_line, 0 })
      end
    end

    self:refresh_signs()
  end)
end

---@param is_visual boolean
function DiffState:mark_action(is_visual)
  local file = self.file
  if not file then
    return
  end
  if self.mode == "all" then
    vim.notify("Switch to Remaining or Reviewed view to mark lines (press t)", vim.log.levels.WARN)
    return
  end
  if file.isBinary then
    vim.notify("Cannot mark binary file", vim.log.levels.WARN)
    return
  end

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

  kjn.set_blob(
    {
      dir = self.dir,
      change_id = self.change_id,
      commit_id = self.commit_id,
      file_path = utils.file_path(file),
    },
    content_str,
    function(err, _)
      if err then
        vim.notify("kjn set-blob: " .. err, vim.log.levels.ERROR)
      end
      if self.callbacks then
        self.callbacks.on_mark()
      end
    end
  )
end

---@param file kenjutu.FileEntry
---@param new_status "reviewed" | "unreviewed"
function DiffState:on_file_toggled(file, new_status)
  local file_path = utils.file_path(file)
  local marker_bufname = diff_buf_name(self.change_id, file_path, "marker")
  local marker_bufnr = vim.fn.bufnr(marker_bufname)
  if marker_bufnr == -1 then
    return
  end
  local new_marker_tree = new_status == "reviewed" and "target" or "base"
  local new_marker_bufnr = vim.fn.bufnr(diff_buf_name(self.change_id, file_path, new_marker_tree))
  if new_marker_bufnr == -1 then
    if vim.fn.bufwinid(marker_bufnr) ~= -1 then
      self:update_wins(true)
    else
      vim.api.nvim_buf_delete(marker_bufnr, { force = true })
    end
    return
  end
  local lines = vim.api.nvim_buf_get_lines(new_marker_bufnr, 0, -1, false)
  vim.bo[marker_bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(marker_bufnr, 0, -1, false, lines)
  vim.bo[marker_bufnr].modifiable = false
end

function DiffState:new_comment()
  local file = self.file
  if not file then
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
    file_path = utils.file_path(file),
    commit_id = self.commit_id,
    dir = self.dir,
    side = tree == "base" and "Old" or "New",
    on_create = function()
      self:refresh_signs()
    end,
  })
end

function DiffState:open_thread_at_cursor()
  local file = self.file
  if not file then
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
  local file_path = utils.file_path(file)

  fetch_file_comments(self.dir, self.change_id, self.commit_id, file_path, function(comments)
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
  end)
end

function DiffState:open_all_comments()
  local comment_picker = require("kenjutu.comment_picker")
  comment_picker.open({
    dir = self.dir,
    change_id = self.change_id,
    commit_id = self.commit_id,
    on_select = function(file_path, pc)
      local cb = self.callbacks
      if not cb then
        return
      end
      cb.navigate_to(file_path, pc.ported_line, pc.comment.side)
    end,
  })
end

function DiffState:open_comment_list()
  local file = self.file
  if not file then
    return
  end
  local file_path = utils.file_path(file)

  fetch_file_comments(self.dir, self.change_id, self.commit_id, file_path, function(comments)
    mod_comments.open_comment_list({
      file_path = file_path,
      comments = comments,
      dir = self.dir,
      change_id = self.change_id,
      on_resolve = function()
        self:refresh_signs()
      end,
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
        elseif self.mode == "all" then
          winnr = side == "New" and self.right_winnr or self.left_winnr
        end
        if winnr and vim.api.nvim_win_is_valid(winnr) then
          vim.api.nvim_set_current_win(winnr)
          vim.api.nvim_win_set_cursor(winnr, { pc.ported_line, 0 })
        end
      end,
    })
  end)
end

function DiffState:refresh_signs()
  kjn.get_comments(self.dir, self.change_id, self.commit_id, function(err, result)
    if err then
      vim.notify("Error loading comments: " .. err, vim.log.levels.ERROR)
      return
    end
    for _, file_comments in ipairs(result and result.files or {}) do
      local base_bufnr = vim.fn.bufnr(diff_buf_name(self.change_id, file_comments.file_path, "base"))
      if base_bufnr ~= -1 then
        mod_comments.place_signs(base_bufnr, file_comments.comments, "Old")
      end
      local target_bufnr = vim.fn.bufnr(diff_buf_name(self.change_id, file_comments.file_path, "target"))
      if target_bufnr ~= -1 then
        mod_comments.place_signs(target_bufnr, file_comments.comments, "New")
      end
    end
  end)
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
  if vim.api.nvim_win_is_valid(self.left_winnr) then
    vim.wo[self.left_winnr].winbar = nil
  end
  self:cleanup()
end

---@param commit_id string
function DiffState:reload(commit_id)
  self.commit_id = commit_id
  ---@type integer[]
  local kept_bufnr = {}
  for _, bufnr in ipairs(self.created_buffers) do
    if vim.api.nvim_buf_is_valid(bufnr) and vim.fn.bufwinid(bufnr) == -1 then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    else
      table.insert(kept_bufnr, bufnr)
    end
  end
  self.created_buffers = kept_bufnr
  self:update_wins(true)
end

function DiffState:cleanup()
  for _, bufnr in ipairs(self.created_buffers) do
    if vim.api.nvim_buf_is_valid(bufnr) then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
  end
end

---@param anchor_winnr integer
---@param dir string
---@param change_id string
---@param commit_id string
---@return kenjutu.DiffState
function M.create(anchor_winnr, dir, change_id, commit_id)
  local state = DiffState:new({
    anchor_winnr = anchor_winnr,
    dir = dir,
    change_id = change_id,
    commit_id = commit_id,
  })
  return state
end

return M
