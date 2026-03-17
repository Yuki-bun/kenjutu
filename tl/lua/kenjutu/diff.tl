local utils = require("kenjutu.utils")
local kjn = require("kenjutu.kjn")
local mod_comments = require("kenjutu.comments")

local record DiffCallbacks
   focus_file_list: function()
   move_selection: function(string)
   close: function()
   on_mark: function()
end

local record SideInfo
   side: string
   tree: string
end

local type FileEntry = kjn.FileEntry

local record DiffState
   left_winnr: integer
   right_winnr: integer
   mode: string
   file: FileEntry
   dir: string
   change_id: string
   commit_id: string
   callbacks: DiffCallbacks
   created_buffers: {integer}

   metamethod __index: DiffState
end

local record DiffStateInitOpts
   anchor_winnr: integer
   dir: string
   change_id: string
   commit_id: string
end

local record M
   type DiffCallbacks = DiffCallbacks
   type SideInfo = SideInfo
   type FileEntry = FileEntry
   type DiffState = DiffState

   create: function(integer, string, string, string): DiffState
end

local function create_scratch_buf(wipe: boolean): integer
   local bufnr: integer = vim.api.nvim_create_buf(false, true)
   vim.bo[bufnr].buftype = "nofile"
   vim.bo[bufnr].bufhidden = wipe and "wipe" or "hide"
   vim.bo[bufnr].swapfile = false
   vim.bo[bufnr].buflisted = false
   vim.bo[bufnr].modifiable = false
   return bufnr
end

local function enable_diff(winnr: integer)
   vim.api.nvim_win_call(winnr, function(): any
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

local function create_layout(anchor_winnr: integer): integer
   local left_bufnr: integer = create_scratch_buf(true)
   local right_bufnr: integer = create_scratch_buf(true)

   vim.api.nvim_set_current_win(anchor_winnr)
   vim.api.nvim_win_set_buf(anchor_winnr, left_bufnr)
   vim.cmd("rightbelow vsplit")
   local right_winnr: integer = vim.api.nvim_get_current_win()
   vim.api.nvim_win_set_buf(right_winnr, right_bufnr)

   return right_winnr
end

local function tree_for_side(mode: string, side: string): string
   if mode == "remaining" then
      return side == "left" and "marker" or "target"
   else
      return side == "left" and "base" or "marker"
   end
end

local function diff_buf_name(change_id: string, file_path: string, tree: string): string
   return "kenjutu://" .. change_id .. "/" .. file_path .. ":" .. tree
end

local function fetch_file_comments(dir: string, change_id: string, commit_id: string, file_path: string, cb: function({kjn.PortedComment}))
   kjn.get_comments(dir, change_id, commit_id, function(err: string, result: kjn.GetCommentsResult)
      if err then
         vim.notify("Error loading comments: " .. err, vim.log.levels.ERROR)
         cb({})
         return
      end
      local files: {kjn.FileComments} = result and result.files or ({} as {kjn.FileComments})
      for _, file_comments in ipairs(files) do
         if file_comments.file_path == file_path then
            cb(file_comments.comments)
            return
         end
      end
      cb({})
   end)
end

local function diff_off_win(winnr: integer)
   if vim.api.nvim_win_is_valid(winnr) then
      vim.api.nvim_win_call(winnr, function(): any
         vim.cmd("diffoff")
      end)
   end
end

function DiffState:new(opts: DiffStateInitOpts): DiffState
   local right_winnr: integer = create_layout(opts.anchor_winnr)

   local obj: DiffState = setmetatable({
      left_winnr = opts.anchor_winnr,
      right_winnr = right_winnr,
      dir = opts.dir,
      change_id = opts.change_id,
      commit_id = opts.commit_id,
      mode = "remaining",
      file = nil,
      callbacks = nil,
      created_buffers = {},
   } as DiffState, { __index = DiffState } as {string:any}) as DiffState
   return obj
end

function DiffState:current_side(): SideInfo
   local winnr: integer = vim.api.nvim_get_current_win()
   local side: string
   if winnr == self.left_winnr then
      side = "left"
   elseif winnr == self.right_winnr then
      side = "right"
   else
      return nil
   end

   return {
      side = side,
      tree = tree_for_side(self.mode, side),
   } as SideInfo
end

function DiffState:buf(side: string): integer
   local winnr: integer = side == "left" and self.left_winnr or self.right_winnr
   return vim.api.nvim_win_get_buf(winnr)
end

function DiffState:set_callbacks(callbacks: DiffCallbacks)
   self.callbacks = callbacks
   local left_bufnr: integer = self:buf("left")
   local right_bufnr: integer = self:buf("right")
   if left_bufnr then
      self:install_keymaps(left_bufnr)
   end
   if right_bufnr then
      self:install_keymaps(right_bufnr)
   end
end

function DiffState:install_keymaps(bufnr: integer)
   local keymap_opts: {string:any} = { buffer = bufnr, silent = true }
   local cb: DiffCallbacks = self.callbacks
   if not cb then
      vim.notify("buffer has been created before callbacks are set", vim.log.levels.WARN)
      return
   end

   vim.keymap.set("n", "<Tab>", function()
      cb.focus_file_list()
   end, keymap_opts)

   vim.keymap.set("n", "s", function()
      self:mark_action(false)
   end, keymap_opts)
   vim.keymap.set("v", "s", function()
      self:mark_action(true)
      vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "n", false)
   end, keymap_opts)

   vim.keymap.set("n", "gj", function()
      cb.move_selection("down")
   end, keymap_opts)

   vim.keymap.set("n", "gk", function()
      cb.move_selection("up")
   end, keymap_opts)

   vim.keymap.set("n", "t", function()
      self:toggle_mode()
   end, keymap_opts)

   vim.keymap.set({ "n", "v" } as any, "gc", function()
      self:new_comment()
   end, keymap_opts)

   vim.keymap.set("n", "go", function()
      self:open_thread_at_cursor()
   end, keymap_opts)

   vim.keymap.set("n", "gC", function()
      self:open_comment_list()
   end, keymap_opts)

   vim.keymap.set("n", "[x", function()
      self:prev_comment()
   end, keymap_opts)

   vim.keymap.set("n", "]x", function()
      self:next_comment()
   end, keymap_opts)

   vim.keymap.set("n", "q", function()
      cb.close()
   end, keymap_opts)
end

function DiffState:set_file(file: FileEntry)
   self.file = file
   self.mode = file.reviewStatus == "reviewed" and "reviewed" or "remaining"
   self:update_wins(false)
end

function DiffState:toggle_mode()
   self.mode = self.mode == "remaining" and "reviewed" or "remaining"
   self:update_wins(false)
end

function DiffState:update_wins(ignore_cache: boolean)
   local file: FileEntry = self.file
   if not file then
      return
   end
   local ft: string = vim.filetype.match({ filename = utils.file_path(file as utils.FileEntry) } as {string:any})

   local function setup_buffer(tree: string, on_loaded: function(any, integer))
      local function get_or_create_buffer(): integer, boolean
         local buf_name: string = diff_buf_name(self.change_id, utils.file_path(file as utils.FileEntry), tree)
         local existing_bufnr: integer = vim.fn.bufnr(buf_name)
         if existing_bufnr ~= -1 then
            return existing_bufnr, true
         end
         local new_bufnr: integer = create_scratch_buf(false)
         table.insert(self.created_buffers, new_bufnr)
         vim.api.nvim_buf_set_name(new_bufnr, buf_name)
         if ft then
            vim.bo[new_bufnr].filetype = ft
         end
         self:install_keymaps(new_bufnr)
         return new_bufnr, false
      end

      local bufnr: integer, cached: boolean = get_or_create_buffer()
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
         file_path = utils.file_path(file as utils.FileEntry),
         old_path = file.status == "renamed" and file.oldPath or nil,
         tree_kind = tree,
      }, function(err: string, content: string)
         if err then
            on_loaded(err, -1)
            return
         end

         local lines: {string} = vim.split(content or "", "\n", { plain = true })
         if #lines > 0 and lines[#lines] == "" then
            table.remove(lines)
         end
         vim.bo[bufnr].modifiable = true
         vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
         vim.bo[bufnr].modifiable = false

         on_loaded(nil, bufnr)
      end)
   end

   local left_tree: string = tree_for_side(self.mode, "left")
   local right_tree: string = tree_for_side(self.mode, "right")

   utils.await_all({
      left = function(cb: function(any, any))
         setup_buffer(left_tree, cb)
      end,
      right = function(cb: function(any, any))
         setup_buffer(right_tree, cb)
      end,
   }, function(err: any, results: {string: any})
      if err then
         vim.notify("Error loading buffers: " .. (err as string), vim.log.levels.ERROR)
         return
      end
      if not results then
         vim.notify("Unexpected error: missing results", vim.log.levels.ERROR)
         return
      end
      local left_bufnr: integer = results.left as integer
      local right_bufnr: integer = results.right as integer

      diff_off_win(self.left_winnr)
      diff_off_win(self.right_winnr)

      vim.api.nvim_win_set_buf(self.left_winnr, left_bufnr)
      vim.api.nvim_win_set_buf(self.right_winnr, right_bufnr)

      enable_diff(self.left_winnr)
      enable_diff(self.right_winnr)

      self:refresh_signs()
   end)
end

function DiffState:mark_action(is_visual: boolean)
   local file: FileEntry = self.file
   if not file then
      return
   end
   if file.isBinary then
      vim.notify("Cannot mark binary file", vim.log.levels.WARN)
      return
   end

   local bufnr: integer = vim.api.nvim_get_current_buf()
   local left_bufnr: integer = self:buf("left")
   local right_bufnr: integer = self:buf("right")
   if not left_bufnr or not right_bufnr then
      return
   end

   local side: string
   if bufnr == left_bufnr then
      side = "left"
   elseif bufnr == right_bufnr then
      side = "right"
   end
   assert(side, "current buffer is not part of the diff panes")

   local marker_bufnr: integer = self.mode == "remaining" and left_bufnr or right_bufnr
   local is_marker: boolean = bufnr == marker_bufnr
   local cmd: string = is_marker and "diffget" or "diffput"

   vim.bo[marker_bufnr].modifiable = true
   if is_visual then
      local v_start: integer = vim.fn.line("v")
      local v_end: integer = vim.fn.line(".")
      if v_start > v_end then
         v_start, v_end = v_end, v_start
      end
      vim.cmd(string.format("%d,%d%s", v_start, v_end, cmd))
   else
      vim.cmd(cmd)
   end
   vim.bo[marker_bufnr].modifiable = false

   local maker_contents: {string} = vim.api.nvim_buf_get_lines(marker_bufnr, 0, -1, false)
   local content_str: string = table.concat(maker_contents, "\n") .. "\n"

   kjn.set_blob(
      {
         dir = self.dir,
         change_id = self.change_id,
         commit_id = self.commit_id,
         file_path = utils.file_path(file as utils.FileEntry),
      },
      content_str,
      function(err: string, _: {string:any})
         if err then
            vim.notify("kjn set-blob: " .. err, vim.log.levels.ERROR)
         end
         if self.callbacks then
            self.callbacks.on_mark()
         end
      end
   )
end

function DiffState:on_file_toggled(file: FileEntry, new_status: string)
   local file_path: string = utils.file_path(file as utils.FileEntry)
   local marker_bufname: string = diff_buf_name(self.change_id, file_path, "marker")
   local marker_bufnr: integer = vim.fn.bufnr(marker_bufname)
   if marker_bufnr == -1 then
      return
   end
   local new_marker_tree: string = new_status == "reviewed" and "target" or "base"
   local new_marker_bufnr: integer = vim.fn.bufnr(diff_buf_name(self.change_id, file_path, new_marker_tree))
   if new_marker_bufnr == -1 then
      if vim.fn.bufwinid(marker_bufnr) ~= -1 then
         self:update_wins(true)
      else
         vim.api.nvim_buf_delete(marker_bufnr, { force = true } as {string:any})
      end
      return
   end
   local lines: {string} = vim.api.nvim_buf_get_lines(new_marker_bufnr, 0, -1, false)
   vim.bo[marker_bufnr].modifiable = true
   vim.api.nvim_buf_set_lines(marker_bufnr, 0, -1, false, lines)
   vim.bo[marker_bufnr].modifiable = false
end

function DiffState:new_comment()
   local file: FileEntry = self.file
   if not file then
      return
   end
   local side_info: SideInfo = self:current_side()
   if not side_info then
      return
   end
   local tree: string = side_info.tree
   if tree == "marker" then
      vim.notify("Cannot comment on the marker version of the file", vim.log.levels.WARN)
      return
   end

   mod_comments.open_new_comment({
      change_id = self.change_id,
      file_path = utils.file_path(file as utils.FileEntry),
      commit_id = self.commit_id,
      dir = self.dir,
      side = tree == "base" and "Old" or "New",
      on_create = function()
         self:refresh_signs()
      end,
   })
end

function DiffState:open_thread_at_cursor()
   local file: FileEntry = self.file
   if not file then
      return
   end
   local side_info: SideInfo = self:current_side()
   if not side_info then
      return
   end
   if side_info.tree == "marker" then
      return
   end

   local side: string = side_info.tree == "base" and "Old" or "New"
   local cursor_line: integer = vim.api.nvim_win_get_cursor(0)[1]
   local file_path: string = utils.file_path(file as utils.FileEntry)

   fetch_file_comments(self.dir, self.change_id, self.commit_id, file_path, function(comments: {kjn.PortedComment})
      local at_line: {kjn.PortedComment} = mod_comments.comments_at_line(comments, cursor_line, side)
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

function DiffState:open_comment_list()
   local file: FileEntry = self.file
   if not file then
      return
   end
   local file_path: string = utils.file_path(file as utils.FileEntry)

   fetch_file_comments(self.dir, self.change_id, self.commit_id, file_path, function(comments: {kjn.PortedComment})
      mod_comments.open_comment_list({
         file_path = file_path,
         comments = comments,
         dir = self.dir,
         change_id = self.change_id,
         on_resolve = function()
            self:refresh_signs()
         end,
         on_select = function(pc: kjn.PortedComment)
            if not pc.ported_line then
               return
            end
            local comment_side: string = pc.comment.side
            local winnr: integer
            if self.mode == "remaining" and comment_side == "New" then
               winnr = self.right_winnr
            elseif self.mode == "reviewed" and comment_side == "Old" then
               winnr = self.left_winnr
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
   kjn.get_comments(self.dir, self.change_id, self.commit_id, function(err: string, result: kjn.GetCommentsResult)
      if err then
         vim.notify("Error loading comments: " .. err, vim.log.levels.ERROR)
         return
      end
      local files: {kjn.FileComments} = result and result.files or ({} as {kjn.FileComments})
      for _, file_comments in ipairs(files) do
         local base_bufnr: integer = vim.fn.bufnr(diff_buf_name(self.change_id, file_comments.file_path, "base"))
         if base_bufnr ~= -1 then
            mod_comments.place_signs(base_bufnr, file_comments.comments, "Old")
         end
         local target_bufnr: integer = vim.fn.bufnr(diff_buf_name(self.change_id, file_comments.file_path, "target"))
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
   self:cleanup()
end

function DiffState:reload(commit_id: string)
   self.commit_id = commit_id
   local kept_bufnr: {integer} = {}
   for _, bufnr in ipairs(self.created_buffers) do
      if vim.api.nvim_buf_is_valid(bufnr) and vim.fn.bufwinid(bufnr) == -1 then
         vim.api.nvim_buf_delete(bufnr, { force = true } as {string:any})
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
         vim.api.nvim_buf_delete(bufnr, { force = true } as {string:any})
      end
   end
end

function M.create(anchor_winnr: integer, dir: string, change_id: string, commit_id: string): DiffState
   local state: DiffState = DiffState:new({
      anchor_winnr = anchor_winnr,
      dir = dir,
      change_id = change_id,
      commit_id = commit_id,
   })
   return state
end

return M
