local kjn = require("kenjutu.kjn")
local utils = require("kenjutu.utils")
local file_render = require("kenjutu.file_render")

local record JjMod
   record CommitMetadata
      summary: string
      description: string
      author: string
      timestamp: string
   end
   fetch_commit_metadata: function(string, string, function(string, JjMod.CommitMetadata))
end
local jj = require("kenjutu.jj") as JjMod

local record M
   record FileTreeState
      bufnr: integer
      winnr: integer
      dir: string
      current_commit_id: string

      metamethod __index: FileTreeState

      new: function(string, integer): FileTreeState
      update: function(FileTreeState, Commit)
      close: function(FileTreeState)
   end
end

local record Commit
   change_id: string
   commit_id: string
end

local record KjnFilesResult
   files: {file_render.FileEntry}
end

local ns = vim.api.nvim_create_namespace("kenjutu_file_tree")

local record AwaitResults
   files: {file_render.FileEntry}
   metadata: JjMod.CommitMetadata
end

local function fetch_commit_data(dir: string, change_id: string, callback: function(string, {file_render.FileEntry}, JjMod.CommitMetadata))
   utils.await_all({
      files = function(cb: function(string, any))
         kjn.files(dir, change_id, function(err: string, result: KjnFilesResult)
            cb(err, not err and result and result.files or nil)
         end)
      end,
      metadata = function(cb: function(string, any))
         jj.fetch_commit_metadata(dir, change_id, cb)
      end,
   } as {string: function(function(string, any))}, function(err: string, results: AwaitResults)
      if err or not results then
         callback(err, {}, nil)
         return
      end
      callback(nil, results.files or {}, results.metadata)
   end)
end

local function format_metadata_lines(metadata: JjMod.CommitMetadata): {file_render.RenderLine}
   if not metadata then
      return {}
   end

   local lines: {file_render.RenderLine} = {}

   if metadata.summary ~= "" then
      local text = " " .. metadata.summary
      table.insert(lines, {
         text = text,
         highlights = { { 0, #text, "KenjutuCommitSummary" } },
      })
   end

   local author_ts: {string} = {}
   local author_ts_hls: {{any}} = {}
   local col = 1
   table.insert(author_ts, " ")

   if metadata.author ~= "" then
      local author = metadata.author
      table.insert(author_ts, author)
      table.insert(author_ts_hls, { col, col + #author, "KenjutuCommitAuthor" })
      col = col + #author
   end

   if metadata.timestamp ~= "" then
      local sep: string = metadata.author ~= "" and "  " or ""
      local ts = sep .. metadata.timestamp
      table.insert(author_ts, ts)
      table.insert(author_ts_hls, { col, col + #ts, "KenjutuCommitTimestamp" })
      col = col + #ts
   end

   if #author_ts > 1 then
      table.insert(lines, {
         text = table.concat(author_ts),
         highlights = author_ts_hls,
      })
   end

   if metadata.description ~= "" then
      table.insert(lines, { text = "", highlights = {} })
      for _, desc_line in ipairs(vim.split(metadata.description, "\n", { plain = true } as {string: any})) do
         local text = " " .. desc_line
         table.insert(lines, {
            text = text,
            highlights = { { 0, #text, "KenjutuCommitDescription" } },
         })
      end
   end

   table.insert(lines, { text = "", highlights = {} })
   return lines
end

local function render(bufnr: integer, files: {file_render.FileEntry}, winnr: integer, metadata: JjMod.CommitMetadata)
   local render_lines: {file_render.RenderLine} = {}

   for _, ml in ipairs(format_metadata_lines(metadata)) do
      table.insert(render_lines, ml)
   end

   local reviewed = file_render.count_reviewed(files)
   local header = string.format(" Files %d/%d", reviewed, #files)
   table.insert(render_lines, { text = header, highlights = { { 0, #header, "KenjutuHeader" } } })
   table.insert(render_lines, { text = "", highlights = {} })

   local tree = file_render.build_tree(files)
   local tree_lines = file_render.flatten_tree(tree, #render_lines + 1)
   vim.list_extend(render_lines, tree_lines)

   file_render.apply_to_buffer(bufnr, render_lines, ns)

   if vim.api.nvim_win_is_valid(winnr) then
      vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
   end
end

local FileTreeState = {} as M.FileTreeState
FileTreeState.__index = FileTreeState

function FileTreeState.new(dir: string, log_winnr: integer): M.FileTreeState
   local prev_win = vim.api.nvim_get_current_win()
   vim.api.nvim_set_current_win(log_winnr)

   vim.cmd("rightbelow vsplit")
   local winnr = vim.api.nvim_get_current_win()
   local bufnr = vim.api.nvim_create_buf(false, true)
   vim.api.nvim_win_set_buf(winnr, bufnr)

   vim.bo[bufnr].buftype = "nofile"
   vim.bo[bufnr].bufhidden = "wipe"
   vim.bo[bufnr].swapfile = false
   vim.bo[bufnr].buflisted = false
   vim.bo[bufnr].filetype = "kenjutu-log-files"
   vim.bo[bufnr].modifiable = false

   vim.wo[winnr].cursorline = false
   vim.wo[winnr].number = false
   vim.wo[winnr].signcolumn = "no"
   vim.wo[winnr].wrap = false
   vim.wo[winnr].winfixwidth = false

   vim.cmd("wincmd =")
   vim.api.nvim_set_current_win(prev_win)

   local state: M.FileTreeState = setmetatable({
      bufnr = bufnr,
      winnr = winnr,
      dir = dir,
      current_commit_id = nil,
   } as M.FileTreeState, { __index = FileTreeState } as metatable<M.FileTreeState>) as M.FileTreeState

   return state
end

function FileTreeState.update(self: M.FileTreeState, commit: Commit)
   if self.current_commit_id == commit.commit_id then
      return
   end
   self.current_commit_id = commit.commit_id

   local bufnr = self.bufnr
   local winnr = self.winnr
   local change_id = commit.change_id

   fetch_commit_data(self.dir, change_id, function(err: string, files: {file_render.FileEntry}, metadata: JjMod.CommitMetadata)
      if err then
         vim.notify("Failed to fetch commit data: " .. err, vim.log.levels.ERROR)
         return
      end
      if self.current_commit_id ~= commit.commit_id then
         return
      end
      render(bufnr, files, winnr, metadata)
   end)
end

function FileTreeState.close(self: M.FileTreeState)
   if vim.api.nvim_win_is_valid(self.winnr) then
      vim.api.nvim_win_close(self.winnr, true)
   end
   if vim.api.nvim_buf_is_valid(self.bufnr) then
      vim.api.nvim_buf_delete(self.bufnr, { force = true })
   end
end

M.FileTreeState = FileTreeState

return M
