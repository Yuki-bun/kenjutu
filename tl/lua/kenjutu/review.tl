local kjn = require("kenjutu.kjn")
local diff = require("kenjutu.diff")
local file_list = require("kenjutu.file_list")
local utils = require("kenjutu.utils")

local record M
   record ReviewStateInitOpts
      dir: string
      change_id: string
      commit_id: string
      file_list_bufnr: integer
      file_list_winnr: integer
      diff_state: diff.DiffState
      log_bufnr: integer
      on_close: function()
   end

   open: function(string, {string:string}, integer, function()): ReviewState
end

local record ReviewState
   dir: string
   change_id: string
   commit_id: string
   files: {kjn.FileEntry}
   line_map: {integer: kjn.FileEntry}
   file_list_bufnr: integer
   file_list_winnr: integer
   diff_state: diff.DiffState
   log_bufnr: integer
   on_close: function()

   metamethod __index: ReviewState
end

function ReviewState.new(opts: M.ReviewStateInitOpts): ReviewState
   local self: ReviewState = setmetatable({} as ReviewState, { __index = ReviewState } as {string:any}) as ReviewState
   self.dir = opts.dir
   self.change_id = opts.change_id
   self.commit_id = opts.commit_id
   self.files = {}
   self.line_map = {}
   self.file_list_bufnr = opts.file_list_bufnr
   self.diff_state = opts.diff_state
   self.file_list_winnr = opts.file_list_winnr
   self.log_bufnr = opts.log_bufnr
   self.on_close = opts.on_close
   return self
end

function ReviewState:update_diff_view()
   local file: kjn.FileEntry = self:selected_file()
   if not file then
      return
   end
   self.diff_state:set_file(file)
end

function ReviewState:move_selection(direction: string)
   if #self.files == 0 or not vim.api.nvim_win_is_valid(self.file_list_winnr) then
      return
   end
   local cur_line: integer = vim.api.nvim_win_get_cursor(self.file_list_winnr)[1]
   local line_count: integer = vim.api.nvim_buf_line_count(self.file_list_bufnr)
   local step: integer = direction == "down" and 1 or -1
   local line: integer = cur_line + step
   while line >= 1 and line <= line_count do
      if self.line_map[line] then
         vim.api.nvim_win_set_cursor(self.file_list_winnr, { line, 0 })
         self:update_diff_view()
         return
      end
      line = line + step
   end
end

function ReviewState:refresh_file_list()
   kjn.files(self.dir, self.change_id, function(err: string, result: kjn.FilesResult)
      if err then
         vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
         return
      end
      if not result or not vim.api.nvim_buf_is_valid(self.file_list_bufnr) then
         return
      end
      local commit_changed: boolean = self.commit_id ~= result.commitId
      self.commit_id = result.commitId
      self.files = result.files or ({} as {kjn.FileEntry})
      self.line_map = file_list.render(self.file_list_bufnr, self.files, self.file_list_winnr) as {integer: kjn.FileEntry}
      if commit_changed then
         self.diff_state:reload(self.commit_id)
      end
   end)
end

function ReviewState:toggle_file_reviewed()
   local file: kjn.FileEntry = self:selected_file()
   if not file then
      return
   end
   local path: string = utils.file_path(file as utils.FileEntry)
   local old_path: string = file.oldPath
   local new_path: string = file.newPath
   local mark_opts: kjn.MarkFileOptions = {
      dir = self.dir,
      change_id = self.change_id,
      commit_id = self.commit_id,
      file_path = path,
   }
   if old_path and new_path and old_path ~= new_path then
      mark_opts.old_path = old_path
   end

   local fn: function(kjn.MarkFileOptions, function(string, {string:any}))
   if file.reviewStatus == "reviewed" then
      fn = kjn.unmark_file
   else
      fn = kjn.mark_file
   end
   fn(mark_opts, function(err: string, _: {string:any})
      if err then
         vim.notify("kjn toggle-reviewed: " .. err, vim.log.levels.ERROR)
         return
      end
      self:refresh_file_list()
      local new_status: string = file.reviewStatus == "reviewed" and "unreviewed" or "reviewed"
      self.diff_state:on_file_toggled(file as diff.FileEntry, new_status)
   end)
end

function ReviewState:close()
   local log_bufnr: integer = self.log_bufnr
   local file_list_bufnr: integer = self.file_list_bufnr

   local anchor_winnr: integer = self.diff_state.left_winnr
   self.diff_state:close()

   if vim.api.nvim_win_is_valid(anchor_winnr) then
      vim.api.nvim_win_close(anchor_winnr, true)
   end

   local win: integer = vim.api.nvim_get_current_win()
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
      vim.api.nvim_buf_delete(file_list_bufnr, { force = true } as {string:any})
   end

   self.on_close()
end

function ReviewState:setup_file_list_keymaps()
   local bufnr: integer = self.file_list_bufnr
   local keymap_opts: {string:any} = { buffer = bufnr, silent = true }

   vim.keymap.set("n", "<CR>", function()
      if vim.api.nvim_win_is_valid(self.diff_state.right_winnr) then
         vim.api.nvim_set_current_win(self.diff_state.right_winnr)
      end
   end, keymap_opts)

   vim.keymap.set("n", "<Space>", function()
      self:toggle_file_reviewed()
   end, keymap_opts)

   vim.keymap.set("n", "r", function()
      self:refresh_file_list()
   end, keymap_opts)

   vim.keymap.set("n", "t", function()
      if self.diff_state then
         self.diff_state:toggle_mode()
      end
   end, keymap_opts)

   vim.keymap.set("n", "q", function()
      self:close()
   end, keymap_opts)
end

function ReviewState:selected_file(): kjn.FileEntry
   if not vim.api.nvim_win_is_valid(self.file_list_winnr) then
      return nil
   end
   local line: integer = vim.api.nvim_win_get_cursor(self.file_list_winnr)[1]
   return self.line_map[line]
end

local function create_scratch_buf(ft: string): integer
   local bufnr: integer = vim.api.nvim_create_buf(false, true)
   vim.bo[bufnr].buftype = "nofile"
   vim.bo[bufnr].bufhidden = "wipe"
   vim.bo[bufnr].swapfile = false
   vim.bo[bufnr].buflisted = false
   vim.bo[bufnr].filetype = ft
   return bufnr
end

function M.open(dir: string, commit: {string:string}, log_bufnr: integer, on_close: function()): ReviewState
   local file_list_bufnr: integer = create_scratch_buf("kenjutu-review-files")

   local cur_win: integer = vim.api.nvim_get_current_win()
   vim.api.nvim_win_set_buf(cur_win, file_list_bufnr)

   vim.cmd("rightbelow vsplit")
   local diff_anchor_winnr: integer = vim.api.nvim_get_current_win()

   local file_list_winnr: integer = cur_win
   vim.api.nvim_set_current_win(file_list_winnr)
   vim.api.nvim_win_set_width(file_list_winnr, 40)

   vim.wo[file_list_winnr].cursorline = true
   vim.wo[file_list_winnr].number = false
   vim.wo[file_list_winnr].relativenumber = false
   vim.wo[file_list_winnr].signcolumn = "no"
   vim.wo[file_list_winnr].wrap = false
   vim.wo[file_list_winnr].winfixwidth = true

   vim.bo[file_list_bufnr].modifiable = true
   vim.api.nvim_buf_set_lines(file_list_bufnr, 0, -1, false, { "Loading..." })
   vim.bo[file_list_bufnr].modifiable = false

   local diff_state: diff.DiffState = diff.create(diff_anchor_winnr, dir, commit.change_id, commit.commit_id)

   local s: ReviewState = ReviewState.new({
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
      move_selection = function(direction: string)
         s:move_selection(direction)
      end,
      close = function()
         s:close()
      end,
      on_mark = function()
         s:refresh_file_list()
      end,
   } as diff.DiffCallbacks)

   vim.api.nvim_set_current_win(file_list_winnr)
   vim.api.nvim_win_set_width(file_list_winnr, 40)

   s:setup_file_list_keymaps()

   local prev_file_path: string = nil
   vim.api.nvim_create_autocmd("CursorMoved", {
      buffer = file_list_bufnr,
      callback = function()
         if not vim.api.nvim_win_is_valid(file_list_winnr) then
            return
         end
         local file: kjn.FileEntry = s:selected_file()
         if not file then
            return
         end
         local path: string = utils.file_path(file as utils.FileEntry)
         if path ~= prev_file_path then
            prev_file_path = path
            s:update_diff_view()
         end
      end,
   } as {string:any})

   local tab_page: integer = vim.api.nvim_tabpage_get_number(0)
   vim.api.nvim_create_autocmd("TabClosed", {
      once = true,
      callback = function(e: {string:any})
         local closed_tab: integer = tonumber(e.match as string) as integer
         if closed_tab == tab_page then
            s.diff_state:cleanup()
         end
      end,
   } as {string:any})

   kjn.files(dir, commit.change_id, function(err: string, result: kjn.FilesResult)
      if err then
         vim.notify("kjn files: " .. err, vim.log.levels.ERROR)
         return
      end
      if not result or not vim.api.nvim_buf_is_valid(file_list_bufnr) then
         return
      end

      assert(type(result.commitId) == "string", "missing commitId in kjn files result")
      s.commit_id = result.commitId
      s.files = result.files or ({} as {kjn.FileEntry})
      s.line_map = file_list.render(s.file_list_bufnr, s.files, s.file_list_winnr) as {integer: kjn.FileEntry}
   end)

   return s
end

return M
