local record JjTreeEntry
   path: string
   status: string
end

local record SquashOpts
   from: string
   into: string
   paths: {string}
   message: string
end

local record JjMod
   record LogResult
      lines: {string}
      highlights: {integer: {HighlightSpan}}
      commits_by_line: {integer: Commit}
      commit_lines: {integer}
   end

   record CommitMetadata
      summary: string
      description: string
      author: string
      timestamp: string
   end

   log: function(string, function(string, JjMod.LogResult))
   fetch_commit_metadata: function(string, string, function(string, JjMod.CommitMetadata))
   describe: function(string, string, string, function(string))
   new_commit: function(string, string, function(string))
   squash: function(string, SquashOpts, function(string))
   list_files: function(string, string, function(string, {JjTreeEntry}))
end

local record HighlightSpan
   col_start: integer
   col_end: integer
   hl_group: string
end

local record Commit
   change_id: string
   commit_id: string
end

local record UtilsMod
   await_all: function({string: function(function(any, any))}, function(any, {string:any}))
end

local record SquashFilesMod
   open: function(integer, {JjTreeEntry}, function({string}))
end

local record FileTreeState
   bufnr: integer
   winnr: integer
   dir: string
   current_commit_id: string
   new: function(string, integer): FileTreeState
   update: function(FileTreeState, Commit)
   close: function(FileTreeState)
end

local record FileTreeMod
   FileTreeState: FileTreeState
end

local jj = require("kenjutu.jj") as JjMod
local utils = require("kenjutu.utils") as UtilsMod
local file_tree_mod = require("kenjutu.file_tree") as FileTreeMod
local FileTreeState = file_tree_mod.FileTreeState

local record M
   open: function(): LogScreenState
end

local ns = vim.api.nvim_create_namespace("kenjutu_log")
local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")

vim.api.nvim_set_hl(0, "KenjutuSquashSource", { bg = "#45475a", bold = true } as {string:any})

local describe_buf_counter: integer = 0

local record SquashState
   source: Commit
   source_line: integer
   paths: {string}
end

local record CommitAtCursorOptions
   cursor_line: integer
   exact: boolean
end

local record LogScreenState
   commits_by_line: {integer: Commit}
   commit_lines: {integer}
   dir: string
   file_tree: FileTreeState
   bufnr: integer
   winnr: integer
   squash_state: SquashState
   __index: LogScreenState

   new: function(): LogScreenState
   goto_next_commit: function(LogScreenState)
   goto_prev_commit: function(LogScreenState)
   commit_at_cursor: function(LogScreenState, CommitAtCursorOptions): Commit
   render: function(LogScreenState, JjMod.LogResult)
   enter_squash_mode: function(LogScreenState)
   cancel_squash_mode: function(LogScreenState)
   highlight_squash_source: function(LogScreenState)
   execute_squash: function(LogScreenState)
   open_squash_message_editor: function(LogScreenState, string, string, {string}, string, string)
   open_squash_file_picker: function(LogScreenState)
   setup_keymaps: function(LogScreenState)
   refresh: function(LogScreenState)
   open_describe: function(LogScreenState)
   close: function(LogScreenState)
   setup_cursor_follow: function(LogScreenState)
end
LogScreenState.__index = LogScreenState

function LogScreenState.new(): LogScreenState
   local bufnr = vim.api.nvim_get_current_buf()
   local winnr = vim.api.nvim_get_current_win()

   vim.bo[bufnr].buftype = "nofile"
   vim.bo[bufnr].bufhidden = "hide"
   vim.bo[bufnr].swapfile = false
   vim.bo[bufnr].buflisted = false
   vim.bo[bufnr].filetype = "kenjutu-log"

   vim.wo[winnr].cursorline = true
   vim.wo[winnr].number = false
   vim.wo[winnr].relativenumber = false
   vim.wo[winnr].signcolumn = "no"
   vim.wo[winnr].wrap = false

   vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "Loading..." })
   vim.bo[bufnr].modifiable = false

   local fields: LogScreenState = {
      commits_by_line = {},
      commit_lines = {},
      dir = vim.fn.getcwd() as string,
      file_tree = nil,
      bufnr = bufnr,
      winnr = winnr,
      squash_state = nil,
   }
   return setmetatable(fields, { __index = LogScreenState }) as LogScreenState
end

function LogScreenState:goto_next_commit()
   local current = (vim.api.nvim_win_get_cursor(0) as {integer})[1]
   for _, line_no in ipairs(self.commit_lines) do
      if line_no > current then
         vim.api.nvim_win_set_cursor(0, { line_no, 0 })
         break
      end
   end
end

function LogScreenState:goto_prev_commit()
   local current = (vim.api.nvim_win_get_cursor(0) as {integer})[1]
   for i = #self.commit_lines, 1, -1 do
      if self.commit_lines[i] < current then
         vim.api.nvim_win_set_cursor(0, { self.commit_lines[i], 0 })
         break
      end
   end
end

function LogScreenState:commit_at_cursor(opts: CommitAtCursorOptions): Commit
   opts = opts or {} as CommitAtCursorOptions
   local cursor_line = opts.cursor_line or (vim.api.nvim_win_get_cursor(self.winnr) as {integer})[1]
   local exact = opts.exact or false
   if not cursor_line then
      cursor_line = (vim.api.nvim_win_get_cursor(0) as {integer})[1]
   end
   if self.commits_by_line[cursor_line] then
      return self.commits_by_line[cursor_line]
   end
   if exact then
      return nil
   end
   local nearest: integer = nil
   for _, line_no in ipairs(self.commit_lines) do
      if line_no <= cursor_line then
         nearest = line_no
      else
         break
      end
   end
   if nearest then
      return self.commits_by_line[nearest]
   end
   return nil
end

function LogScreenState:render(result: JjMod.LogResult)
   local current_commit = self:commit_at_cursor()
   local prev_change_id: string = current_commit and current_commit.change_id or nil

   local bufnr = self.bufnr
   vim.bo[bufnr].modifiable = true
   vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, result.lines)
   vim.bo[bufnr].modifiable = false

   self.commits_by_line = result.commits_by_line
   self.commit_lines = result.commit_lines

   vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
   if result.highlights then
      for line_idx, spans in pairs(result.highlights) do
         for _, span in ipairs(spans) do
            pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, line_idx - 1, span.col_start, {
               end_col = span.col_end,
               hl_group = span.hl_group,
            } as {string:any})
         end
      end
   end

   local restored = false
   if prev_change_id then
      for _, line_no in ipairs(result.commit_lines) do
         if result.commits_by_line[line_no].change_id == prev_change_id then
            vim.api.nvim_win_set_cursor(0, { line_no, 0 })
            restored = true
            break
         end
      end
   end
   if not restored then
      local first_commit_line = result.commit_lines[1]
      if first_commit_line then
         vim.api.nvim_win_set_cursor(0, { first_commit_line, 0 })
      end
   end
end

function LogScreenState:enter_squash_mode()
   local cursor_line = (vim.api.nvim_win_get_cursor(0) as {integer})[1]
   local commit = self:commit_at_cursor({ exact = true } as CommitAtCursorOptions)
   if not commit then
      return
   end
   local state: SquashState = {
      source = commit,
      source_line = cursor_line,
      paths = nil,
   }
   self.squash_state = state
   self:highlight_squash_source()
   vim.api.nvim_echo({ { "Squash: select destination (s to confirm, <Esc> to cancel)", "WarningMsg" } }, false, {})
end

function LogScreenState:cancel_squash_mode()
   self.squash_state = nil
   vim.api.nvim_buf_clear_namespace(self.bufnr, squash_ns, 0, -1)
   vim.api.nvim_echo({ { "" } }, false, {})
end

function LogScreenState:highlight_squash_source()
   vim.api.nvim_buf_clear_namespace(self.bufnr, squash_ns, 0, -1)
   local squash_state = self.squash_state
   if not squash_state then
      return
   end
   local line_idx = squash_state.source_line - 1
   local line_text = (vim.api.nvim_buf_get_lines(self.bufnr, line_idx, line_idx + 1, false) as {string})[1] or ""
   pcall(vim.api.nvim_buf_set_extmark, self.bufnr, squash_ns, line_idx, 0, {
      end_col = #line_text,
      hl_group = "KenjutuSquashSource",
   } as {string:any})
end

local function resolve_squash_message(source_desc: string, dest_desc: string): string
   local has_source = source_desc ~= ""
   local has_dest = dest_desc ~= ""
   if has_source and has_dest then
      return nil
   end
   if has_source then
      return source_desc
   end
   if has_dest then
      return dest_desc
   end
   return nil
end

local function full_description(metadata: JjMod.CommitMetadata): string
   if metadata.description ~= "" then
      return metadata.summary .. "\n" .. metadata.description
   end
   return metadata.summary
end

function LogScreenState:execute_squash()
   local dest = self:commit_at_cursor()
   if not dest then
      return
   end
   if dest.change_id == self.squash_state.source.change_id then
      vim.notify("Cannot squash a commit into itself", vim.log.levels.WARN)
      return
   end
   local source = self.squash_state.source
   if not source then
      return
   end

   local paths = self.squash_state.paths
   self:cancel_squash_mode()

   local source_change_id = source.change_id
   local dest_change_id = dest.change_id

   local self_ref = self
   utils.await_all({
      source = function(cb: function(any, any))
         jj.fetch_commit_metadata(self_ref.dir, source_change_id, cb as function(string, JjMod.CommitMetadata))
      end,
      dest = function(cb: function(any, any))
         jj.fetch_commit_metadata(self_ref.dir, dest_change_id, cb as function(string, JjMod.CommitMetadata))
      end,
   }, function(err: any, results: {string:any})
      if err or not results then
         vim.notify("jj squash: failed to fetch commit metadata", vim.log.levels.ERROR)
         return
      end

      local source_desc = full_description(results.source as JjMod.CommitMetadata)
      local dest_desc = full_description(results.dest as JjMod.CommitMetadata)
      local message = resolve_squash_message(source_desc, dest_desc)

      if message ~= nil or (source_desc == "" and dest_desc == "") then
         jj.squash(self_ref.dir, {
            from = source_change_id,
            into = dest_change_id,
            paths = paths,
            message = message,
         } as SquashOpts, function(squash_err: string)
            if squash_err then
               vim.notify("jj squash: " .. squash_err, vim.log.levels.ERROR)
               return
            end
            vim.notify("Squashed into " .. dest_change_id:sub(1, 8), vim.log.levels.INFO)
            self_ref:refresh()
         end)
         return
      end

      self_ref:open_squash_message_editor(source_change_id, dest_change_id, paths, source_desc, dest_desc)
   end)
end

function LogScreenState:open_squash_message_editor(source_change_id: string, dest_change_id: string, paths: {string}, source_desc: string, dest_desc: string)
   local content_lines: {string} = {}
   table.insert(content_lines, "JJ: Enter a description for the combined commit.")
   table.insert(content_lines, "JJ: Description from the destination commit:")
   for _, line in ipairs(vim.split(dest_desc, "\n", { plain = true } as {string:any})) do
      table.insert(content_lines, line)
   end
   table.insert(content_lines, "")
   table.insert(content_lines, "JJ: Description from source commit:")
   for _, line in ipairs(vim.split(source_desc, "\n", { plain = true } as {string:any})) do
      table.insert(content_lines, line)
   end

   vim.api.nvim_set_current_win(self.winnr)
   vim.cmd("aboveleft split")
   local desc_winnr = vim.api.nvim_get_current_win()
   local desc_bufnr = vim.api.nvim_create_buf(false, true)
   vim.api.nvim_win_set_buf(desc_winnr, desc_bufnr)

   vim.bo[desc_bufnr].buftype = "acwrite"
   vim.bo[desc_bufnr].filetype = "jjdescription"
   vim.bo[desc_bufnr].swapfile = false
   vim.bo[desc_bufnr].buflisted = false
   describe_buf_counter = describe_buf_counter + 1
   vim.api.nvim_buf_set_name(
      desc_bufnr,
      "squash://" .. source_change_id:sub(1, 8) .. "->" .. dest_change_id:sub(1, 8) .. "/" .. describe_buf_counter
   )

   vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, content_lines)
   vim.bo[desc_bufnr].modified = false

   local self_ref = self
   vim.api.nvim_create_autocmd("BufWriteCmd", {
      buffer = desc_bufnr,
      callback = function()
         local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false) as {string}
         local filtered: {string} = {}
         for _, line in ipairs(lines) do
            if not vim.startswith(line, "JJ: ") then
               table.insert(filtered, line)
            end
         end
         local message = table.concat(filtered, "\n")

         jj.squash(self_ref.dir, {
            from = source_change_id,
            into = dest_change_id,
            paths = paths,
            message = message,
         } as SquashOpts, function(err: string)
            if err then
               vim.notify("jj squash: " .. err, vim.log.levels.ERROR)
               return
            end

            vim.bo[desc_bufnr].modified = false
            if vim.api.nvim_win_is_valid(desc_winnr) then
               vim.api.nvim_win_close(desc_winnr, true)
            end
            if vim.api.nvim_buf_is_valid(desc_bufnr) then
               vim.api.nvim_buf_delete(desc_bufnr, { force = true })
            end

            vim.notify("Squashed into " .. dest_change_id:sub(1, 8), vim.log.levels.INFO)
            self_ref:refresh()
         end)
      end,
   } as {string:any})

   vim.keymap.set("n", "q", function()
      if vim.api.nvim_win_is_valid(desc_winnr) then
         vim.api.nvim_win_close(desc_winnr, true)
      end
      if vim.api.nvim_buf_is_valid(desc_bufnr) then
         vim.api.nvim_buf_delete(desc_bufnr, { force = true })
      end
   end, { buffer = desc_bufnr, silent = true })
end

function LogScreenState:open_squash_file_picker()
   local commit = self:commit_at_cursor()
   if not commit then
      return
   end

   local self_ref = self
   jj.list_files(self.dir, commit.change_id, function(err: string, files: {JjTreeEntry})
      if err or not files then
         vim.notify("kjn files: " .. (err or "unknown error"), vim.log.levels.ERROR)
         return
      end

      if #files == 0 then
         vim.notify("No files in commit", vim.log.levels.WARN)
         return
      end

      local squash_files = require("kenjutu.squash_files") as SquashFilesMod
      squash_files.open(self_ref.winnr, files, function(selected_paths: {string})
         if not selected_paths or #selected_paths == 0 then
            return
         end
         self_ref:enter_squash_mode()
         self_ref.squash_state.paths = selected_paths
      end)
   end)
end

function LogScreenState:setup_keymaps()
   local bufnr = self.bufnr
   local opts = { buffer = bufnr, silent = true }

   local self_ref = self

   vim.keymap.set("n", "j", function()
      self_ref:goto_next_commit()
   end, opts)

   vim.keymap.set("n", "k", function()
      self_ref:goto_prev_commit()
   end, opts)

   local function on_close_review()
      self_ref.file_tree = FileTreeState.new(self_ref.dir, self_ref.winnr)
      local commit = self_ref:commit_at_cursor()
      if commit then
         self_ref.file_tree:update(commit)
      end
   end

   vim.keymap.set("n", "<CR>", function()
      local commit = self_ref:commit_at_cursor({ exact = true } as CommitAtCursorOptions)
      if commit then
         if self_ref.file_tree then
            self_ref.file_tree:close()
            self_ref.file_tree = nil
         end
         local review = require("kenjutu.review") as {string:any}
         local open_fn = review.open as function(string, Commit, integer, function())
         open_fn(self_ref.dir, commit, bufnr, on_close_review)
      end
   end, opts)

   vim.keymap.set("n", "r", function()
      self_ref:refresh()
   end, opts)

   vim.keymap.set("n", "n", function()
      local commit = self_ref:commit_at_cursor({ exact = true } as CommitAtCursorOptions)
      if not commit then
         return
      end
      jj.new_commit(self_ref.dir, commit.change_id, function(err: string)
         if err then
            vim.notify("jj new: " .. err, vim.log.levels.ERROR)
            return
         end
         self_ref:refresh()
      end)
   end, opts)

   vim.keymap.set("n", "d", function()
      self_ref:open_describe()
   end, opts)

   vim.keymap.set("n", "s", function()
      local cur = (vim.api.nvim_win_get_cursor(0) as {integer})[1]
      local commit = self_ref.commits_by_line[cur]
      if not commit then
         return
      end

      if self_ref.squash_state then
         if commit.change_id == self_ref.squash_state.source.change_id then
            self_ref:cancel_squash_mode()
            return
         end
         self_ref:execute_squash()
      else
         self_ref:enter_squash_mode()
      end
   end, opts)

   vim.keymap.set("n", "S", function()
      if self_ref.squash_state then
         self_ref:cancel_squash_mode()
      end
      self_ref:open_squash_file_picker()
   end, opts)

   vim.keymap.set("n", "<Esc>", function()
      if self_ref.squash_state then
         self_ref:cancel_squash_mode()
      end
   end, opts)

   vim.keymap.set("n", "q", function()
      self_ref:close()
   end, opts)
end

function LogScreenState:refresh()
   local self_ref = self
   jj.log(self.dir, function(err: string, result: JjMod.LogResult)
      if err or result == nil then
         vim.notify("jj log: " .. err, vim.log.levels.ERROR)
         return
      end
      if not vim.api.nvim_buf_is_valid(self_ref.bufnr) then
         return
      end
      self_ref:render(result)
      if self_ref.file_tree then
         local commit = self_ref:commit_at_cursor()
         if commit then
            self_ref.file_tree:update(commit)
         end
      end
   end)
end

function LogScreenState:open_describe()
   local commit = self:commit_at_cursor()
   if not commit then
      return
   end

   local change_id = commit.change_id

   local self_ref = self
   jj.fetch_commit_metadata(self.dir, change_id, function(err: string, metadata: JjMod.CommitMetadata)
      if err or metadata == nil then
         vim.notify("jj log metadata: " .. (err or "unknown error"), vim.log.levels.ERROR)
         return
      end

      local full_desc = metadata.summary
      if metadata.description ~= "" then
         full_desc = full_desc .. "\n" .. metadata.description
      end

      vim.api.nvim_set_current_win(self_ref.winnr)
      vim.cmd("aboveleft split")
      local desc_winnr = vim.api.nvim_get_current_win()
      local desc_bufnr = vim.api.nvim_create_buf(false, true)
      vim.api.nvim_win_set_buf(desc_winnr, desc_bufnr)

      vim.bo[desc_bufnr].buftype = "acwrite"
      vim.bo[desc_bufnr].filetype = "jjdescription"
      vim.bo[desc_bufnr].swapfile = false
      vim.bo[desc_bufnr].buflisted = false
      describe_buf_counter = describe_buf_counter + 1
      vim.api.nvim_buf_set_name(desc_bufnr, "describe://" .. change_id:sub(1, 8) .. "/" .. describe_buf_counter)

      local desc_lines = vim.split(full_desc, "\n", { plain = true } as {string:any})
      vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, desc_lines)
      vim.bo[desc_bufnr].modified = false

      vim.api.nvim_create_autocmd("BufWriteCmd", {
         buffer = desc_bufnr,
         callback = function()
            local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false) as {string}
            local message = table.concat(lines, "\n")

            jj.describe(self_ref.dir, change_id, message, function(desc_err: string)
               if desc_err then
                  vim.notify("jj describe: " .. desc_err, vim.log.levels.ERROR)
                  return
               end

               vim.bo[desc_bufnr].modified = false
               if vim.api.nvim_win_is_valid(desc_winnr) then
                  vim.api.nvim_win_close(desc_winnr, true)
               end
               if vim.api.nvim_buf_is_valid(desc_bufnr) then
                  vim.api.nvim_buf_delete(desc_bufnr, { force = true })
               end

               self_ref:refresh()
            end)
         end,
      } as {string:any})

      vim.keymap.set("n", "q", function()
         if vim.api.nvim_win_is_valid(desc_winnr) then
            vim.api.nvim_win_close(desc_winnr, true)
         end
         if vim.api.nvim_buf_is_valid(desc_bufnr) then
            vim.api.nvim_buf_delete(desc_bufnr, { force = true })
         end
      end, { buffer = desc_bufnr, silent = true })
   end)
end

function LogScreenState:close()
   if self.file_tree then
      self.file_tree:close()
      self.file_tree = nil
   end
   local tab_count = #(vim.api.nvim_list_tabpages() as {integer})
   if tab_count > 1 then
      vim.cmd("tabclose")
   elseif vim.api.nvim_buf_is_valid(self.bufnr) then
      vim.api.nvim_buf_delete(self.bufnr, { force = true })
   end
end

function LogScreenState:setup_cursor_follow()
   local bufnr = self.bufnr
   local self_ref = self
   vim.api.nvim_create_autocmd("CursorMoved", {
      buffer = bufnr,
      callback = function()
         local commit = self_ref:commit_at_cursor()
         if not commit then
            return
         end

         if not self_ref.file_tree then
            self_ref.file_tree = FileTreeState.new(self_ref.dir, self_ref.winnr)
         end

         self_ref.file_tree:update(commit)
      end,
   } as {string:any})
end

function M.open(): LogScreenState
   vim.cmd("tabnew")
   local s = LogScreenState.new()

   s.file_tree = FileTreeState.new(s.dir, s.winnr)
   vim.api.nvim_set_current_win(s.winnr)

   s:setup_keymaps()
   s:setup_cursor_follow()

   jj.log(s.dir, function(err: string, result: JjMod.LogResult)
      if err or result == nil then
         vim.notify("jj log: " .. err, vim.log.levels.ERROR)
         return
      end
      if not vim.api.nvim_buf_is_valid(s.bufnr) then
         return
      end
      s:render(result)
   end)

   vim.api.nvim_create_autocmd("BufWipeout", {
      buffer = s.bufnr,
      once = true,
      callback = function()
         s:close()
      end,
   } as {string:any})

   return s
end

return M
