local record Daemon
   job_id: integer
   next_id: integer
   pending: {integer: function(string, {string:any})}
   buf: string
end

local record FetchBlobOptions
   change_id: string
   commit_id: string
   file_path: string
   old_path: string
   tree_kind: string
   dir: string
end

local record FilesResult
   files: {FileEntry}
   commitId: string
   changeId: string
end

local record FileEntry
   oldPath: string
   newPath: string
   status: string
   additions: integer
   deletions: integer
   isBinary: boolean
   reviewStatus: string
end

local record SetBlobOptions
   dir: string
   change_id: string
   commit_id: string
   file_path: string
end

local record MarkFileOptions
   dir: string
   change_id: string
   commit_id: string
   file_path: string
   old_path: string
end

local record AddCommentOptions
   dir: string
   change_id: string
   commit_id: string
   file_path: string
   side: string
   line: integer
   start_line: integer
   body: string
end

local record ReplyToCommentOptions
   dir: string
   change_id: string
   file_path: string
   parent_comment_id: string
   body: string
end

local record EditCommentOptions
   dir: string
   change_id: string
   file_path: string
   comment_id: string
   body: string
end

local record ResolveCommentOptions
   dir: string
   change_id: string
   file_path: string
   comment_id: string
end

local record MaterializedReply
   id: string
   body: string
   created_at: string
   updated_at: string
   edit_count: integer
end

local record MaterializedComment
   id: string
   target_sha: string
   side: string
   line: integer
   start_line: integer
   body: string
   anchor: Anchor
   resolved: boolean
   created_at: string
   updated_at: string
   edit_count: integer
   replies: {MaterializedReply}
end

local record Anchor
   before: {string}
   target: {string}
   after: {string}
end

local record PortedComment
   comment: MaterializedComment
   ported_line: integer
   ported_start_line: integer
   is_ported: boolean
end

local record FileComments
   file_path: string
   comments: {PortedComment}
end

local record GetCommentsResult
   files: {FileComments}
end

local record M
   type FetchBlobOptions = FetchBlobOptions
   type FilesResult = FilesResult
   type FileEntry = FileEntry
   type SetBlobOptions = SetBlobOptions
   type MarkFileOptions = MarkFileOptions
   type AddCommentOptions = AddCommentOptions
   type ReplyToCommentOptions = ReplyToCommentOptions
   type EditCommentOptions = EditCommentOptions
   type ResolveCommentOptions = ResolveCommentOptions
   type MaterializedReply = MaterializedReply
   type MaterializedComment = MaterializedComment
   type Anchor = Anchor
   type PortedComment = PortedComment
   type FileComments = FileComments
   type GetCommentsResult = GetCommentsResult

   fetch_blob: function(FetchBlobOptions, function(string, string))
   files: function(string, string, function(string, FilesResult))
   set_blob: function(SetBlobOptions, string, function(string, {string:any}))
   mark_file: function(MarkFileOptions, function(string, {string:any}))
   unmark_file: function(MarkFileOptions, function(string, {string:any}))
   get_comments: function(string, string, string, function(string, GetCommentsResult))
   add_comment: function(AddCommentOptions, function(string, {string:any}))
   reply_to_comment: function(ReplyToCommentOptions, function(string, {string:any}))
   edit_comment: function(EditCommentOptions, function(string, {string:any}))
   resolve_comment: function(ResolveCommentOptions, function(string, {string:any}))
   unresolve_comment: function(ResolveCommentOptions, function(string, {string:any}))
   shutdown: function()
end

local kjn_bin: string

local function get_kjn_bin(): string
   if not kjn_bin then
      kjn_bin = require("kenjutu.install").bin_path()
   end
   return kjn_bin
end

local function deep_convert_nil(tbl: {string:any})
   for k, v in pairs(tbl) do
      if v == vim.NIL then
         tbl[k] = nil
      elseif type(v) == "table" then
         deep_convert_nil(v as {string:any})
      end
   end
end

local daemons: {string: Daemon} = {}

local function get_or_start_daemon(dir: string): Daemon
   local existing = daemons[dir]
   if existing then
      return existing
   end

   local daemon: Daemon = {
      job_id = -1,
      next_id = 1,
      pending = {},
      buf = "",
   }

   local bin = get_kjn_bin()
   local job_id = vim.fn.jobstart({ bin, "--dir", dir }, {
      on_stdout = function(_: integer, data: {string}, _: string)
         daemon.buf = daemon.buf .. data[1]
         for i = 2, #data do
            local line = daemon.buf
            daemon.buf = data[i]
            if line ~= "" then
               vim.schedule(function()
                  local ok, resp = pcall(vim.fn.json_decode as function(string): any, line)
                  if not ok or type(resp) ~= "table" then
                     return
                  end
                  local r = resp as {string:any}
                  local id = r.id as integer
                  local cb = daemon.pending[id]
                  if not cb then
                     return
                  end
                  daemon.pending[id] = nil
                  if r.error then
                     cb(r.error as string, nil)
                  else
                     local result = r.result
                     if type(result) == "table" then
                        deep_convert_nil(result as {string:any})
                     end
                     cb(nil, result as {string:any})
                  end
               end)
            end
         end
      end,
      on_stderr = function(_: integer, data: {string}, _: string)
         for _, chunk in ipairs(data) do
            if chunk ~= "" then
               vim.schedule(function()
                  vim.notify("kjn daemon: " .. chunk, vim.log.levels.WARN)
               end)
            end
         end
      end,
      on_exit = function(_: integer, _: integer, _: string)
         local pending = daemon.pending
         daemon.pending = {}
         daemons[dir] = nil
         for _, cb in pairs(pending) do
            cb("kjn daemon exited", nil)
         end
      end,
      stdout_buffered = false,
      stderr_buffered = false,
   } as {string:any}) as integer

   if job_id <= 0 then
      error("failed to start kjn daemon: " .. bin)
   end

   daemon.job_id = job_id
   daemons[dir] = daemon
   return daemon
end

local function send_request(dir: string, method: string, params: {string:any}, callback: function(string, {string:any}))
   local daemon = get_or_start_daemon(dir)
   local id = daemon.next_id
   daemon.next_id = id + 1
   daemon.pending[id] = callback

   local req = vim.fn.json_encode({ id = id, method = method, params = params })
   vim.fn.chansend(daemon.job_id, req .. "\n")
end

function M.fetch_blob(opts: FetchBlobOptions, cb: function(string, string))
   local params: {string:any} = {
      change_id = opts.change_id,
      commit = opts.commit_id,
      file = opts.file_path,
      tree = opts.tree_kind,
   }
   if opts.old_path and opts.old_path ~= opts.file_path then
      params.old_path = opts.old_path
   end
   send_request(opts.dir, "blob", params, function(err: string, result: {string:any})
      if err then
         cb(err, nil)
         return
      end
      if not result or not result.content then
         cb(nil, "")
         return
      end
      cb(nil, result.content as string)
   end)
end

function M.files(dir: string, change_id: string, cb: function(string, FilesResult))
   send_request(dir, "files", { change_id = change_id }, cb as function(string, {string:any}))
end

function M.set_blob(opts: SetBlobOptions, content: string, cb: function(string, {string:any}))
   send_request(opts.dir, "set-blob", {
      change_id = opts.change_id,
      commit = opts.commit_id,
      file = opts.file_path,
      content = content,
   }, cb)
end

function M.mark_file(opts: MarkFileOptions, cb: function(string, {string:any}))
   local params: {string:any} = {
      change_id = opts.change_id,
      commit = opts.commit_id,
      file = opts.file_path,
   }
   if opts.old_path and opts.old_path ~= opts.file_path then
      params.old_path = opts.old_path
   end
   send_request(opts.dir, "mark-file", params, cb)
end

function M.unmark_file(opts: MarkFileOptions, cb: function(string, {string:any}))
   local params: {string:any} = {
      change_id = opts.change_id,
      commit = opts.commit_id,
      file = opts.file_path,
   }
   if opts.old_path and opts.old_path ~= opts.file_path then
      params.old_path = opts.old_path
   end
   send_request(opts.dir, "unmark-file", params, cb)
end

function M.get_comments(dir: string, change_id: string, commit_id: string, cb: function(string, GetCommentsResult))
   send_request(dir, "get-comments", {
      change_id = change_id,
      commit = commit_id,
   }, cb as function(string, {string:any}))
end

function M.add_comment(opts: AddCommentOptions, cb: function(string, {string:any}))
   send_request(opts.dir, "add-comment", {
      change_id = opts.change_id,
      commit = opts.commit_id,
      file = opts.file_path,
      side = opts.side,
      line = opts.line,
      start_line = opts.start_line,
      body = opts.body,
   }, cb)
end

function M.reply_to_comment(opts: ReplyToCommentOptions, cb: function(string, {string:any}))
   send_request(opts.dir, "reply-to-comment", {
      change_id = opts.change_id,
      file = opts.file_path,
      parent_comment_id = opts.parent_comment_id,
      body = opts.body,
   }, cb)
end

function M.edit_comment(opts: EditCommentOptions, cb: function(string, {string:any}))
   send_request(opts.dir, "edit-comment", {
      change_id = opts.change_id,
      file = opts.file_path,
      comment_id = opts.comment_id,
      body = opts.body,
   }, cb)
end

function M.resolve_comment(opts: ResolveCommentOptions, cb: function(string, {string:any}))
   send_request(opts.dir, "resolve-comment", {
      change_id = opts.change_id,
      file = opts.file_path,
      comment_id = opts.comment_id,
   }, cb)
end

function M.unresolve_comment(opts: ResolveCommentOptions, cb: function(string, {string:any}))
   send_request(opts.dir, "unresolve-comment", {
      change_id = opts.change_id,
      file = opts.file_path,
      comment_id = opts.comment_id,
   }, cb)
end

function M.shutdown()
   for dir, daemon in pairs(daemons) do
      vim.fn.jobstop(daemon.job_id)
      daemons[dir] = nil
   end
end

vim.api.nvim_create_autocmd("VimLeavePre", {
   callback = function()
      M.shutdown()
   end,
})

return M
