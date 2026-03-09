---@alias kenjutu.TreeKind "base" | "marker" | "target"

---@class kenjutu.Kjn
local M = {}

---@type string?
local kjn_bin

local function get_kjn_bin()
  if not kjn_bin then
    kjn_bin = require("kenjutu.install").bin_path()
  end
  return kjn_bin
end

--- Recursively convert vim.NIL values to native nil in a table.
--- vim.fn.json_decode turns JSON null into vim.NIL; this normalizes
--- the parsed result so downstream code can use plain nil checks.
---@param tbl table
local function deep_convert_nil(tbl)
  for k, v in pairs(tbl) do
    if v == vim.NIL then
      tbl[k] = nil
    elseif type(v) == "table" then
      deep_convert_nil(v)
    end
  end
end

-- ── Daemon management ──────────────────────────────────────────────

---@class kenjutu.Daemon
---@field job_id integer
---@field next_id integer
---@field pending table<integer, fun(err: string|nil, result: table|nil)>
---@field buf string

---@type table<string, kenjutu.Daemon>
local daemons = {}

---@param dir string
---@return kenjutu.Daemon
local function get_or_start_daemon(dir)
  local existing = daemons[dir]
  if existing then
    return existing
  end

  ---@type kenjutu.Daemon
  local daemon = {
    job_id = -1,
    next_id = 1,
    pending = {},
    buf = "",
  }

  local bin = get_kjn_bin()
  local job_id = vim.fn.jobstart({ bin, "--dir", dir }, {
    on_stdout = function(_, data, _)
      daemon.buf = daemon.buf .. data[1]
      for i = 2, #data do
        local line = daemon.buf
        daemon.buf = data[i]
        if line ~= "" then
          vim.schedule(function()
            local ok, resp = pcall(vim.fn.json_decode, line)
            if not ok or type(resp) ~= "table" then
              return
            end
            local id = resp.id
            local cb = daemon.pending[id]
            if not cb then
              return
            end
            daemon.pending[id] = nil
            if resp.error then
              cb(resp.error, nil)
            else
              local result = resp.result
              if type(result) == "table" then
                deep_convert_nil(result)
              end
              cb(nil, result)
            end
          end)
        end
      end
    end,
    on_stderr = function(_, data, _)
      for _, chunk in ipairs(data) do
        if chunk ~= "" then
          vim.schedule(function()
            vim.notify("kjn daemon: " .. chunk, vim.log.levels.WARN)
          end)
        end
      end
    end,
    on_exit = function(_, _, _)
      local pending = daemon.pending
      daemon.pending = {}
      daemons[dir] = nil
      for _, cb in pairs(pending) do
        cb("kjn daemon exited", nil)
      end
    end,
    stdout_buffered = false,
    stderr_buffered = false,
  })

  if job_id <= 0 then
    error("failed to start kjn daemon: " .. bin)
  end

  daemon.job_id = job_id
  daemons[dir] = daemon
  return daemon
end

---@param dir string
---@param method string
---@param params table
---@param callback fun(err: string|nil, result: table|nil)
local function send_request(dir, method, params, callback)
  local daemon = get_or_start_daemon(dir)
  local id = daemon.next_id
  daemon.next_id = id + 1
  daemon.pending[id] = callback

  local req = vim.fn.json_encode({ id = id, method = method, params = params })
  vim.fn.chansend(daemon.job_id, req .. "\n")
end

-- ── Public API (same signatures as before) ─────────────────────────

---@class kenjutu.FetchBlobOptions
---@field change_id string
---@field commit_id string
---@field file_path string
---@field old_path string|nil
---@field tree_kind kenjutu.TreeKind
---@field dir string

---@param opts kenjutu.FetchBlobOptions
---@param cb fun(err: string|nil, content: string|nil)
function M.fetch_blob(opts, cb)
  local params = {
    change_id = opts.change_id,
    commit = opts.commit_id,
    file = opts.file_path,
    tree = opts.tree_kind,
  }
  if opts.old_path and opts.old_path ~= opts.file_path then
    params.old_path = opts.old_path
  end
  send_request(opts.dir, "blob", params, function(err, result)
    if err then
      cb(err, nil)
      return
    end
    if not result or not result.content then
      cb(nil, "")
      return
    end
    cb(nil, result.content)
  end)
end

---@class kenjutu.FilesResult
---@field files kenjutu.FileEntry[]
---@field commitId string
---@field changeId string

---@class kenjutu.FileEntry
---@field oldPath string|nil
---@field newPath string|nil
---@field status string "added"|"modified"|"deleted"|"renamed"|"copied"|"typechange"
---@field additions integer
---@field deletions integer
---@field isBinary boolean
---@field reviewStatus "reviewed"|"partiallyReviewed"|"unreviewed"|"reviewedReverted"

---@param dir string
---@param change_id string
---@param cb fun(err: string|nil, result: kenjutu.FilesResult|nil)
function M.files(dir, change_id, cb)
  send_request(dir, "files", { change_id = change_id }, cb)
end

---@class kenjutu.SetBlobOptions
---@field dir string
---@field change_id string
---@field commit_id string
---@field file_path string

---@param opts kenjutu.SetBlobOptions
---@param content string
---@param cb fun(err: string|nil, result: table|nil)
function M.set_blob(opts, content, cb)
  send_request(opts.dir, "set-blob", {
    change_id = opts.change_id,
    commit = opts.commit_id,
    file = opts.file_path,
    content = content,
  }, cb)
end

---@class kenjutu.MarkFileOptions
---@field dir string
---@field change_id string
---@field commit_id string
---@field file_path string
---@field old_path string|nil

---@param opts kenjutu.MarkFileOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.mark_file(opts, cb)
  local params = {
    change_id = opts.change_id,
    commit = opts.commit_id,
    file = opts.file_path,
  }
  if opts.old_path and opts.old_path ~= opts.file_path then
    params.old_path = opts.old_path
  end
  send_request(opts.dir, "mark-file", params, cb)
end

---@param opts kenjutu.MarkFileOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.unmark_file(opts, cb)
  local params = {
    change_id = opts.change_id,
    commit = opts.commit_id,
    file = opts.file_path,
  }
  if opts.old_path and opts.old_path ~= opts.file_path then
    params.old_path = opts.old_path
  end
  send_request(opts.dir, "unmark-file", params, cb)
end

---@class kenjutu.PortedComment
---@field comment kenjutu.MaterializedComment
---@field ported_line integer|nil
---@field ported_start_line integer|nil
---@field is_ported boolean

---@class kenjutu.MaterializedComment
---@field id string
---@field target_sha string
---@field side "Old"|"New"
---@field line integer
---@field start_line integer|nil
---@field body string
---@field anchor { before: string[], target: string[], after: string[] }
---@field resolved boolean
---@field created_at string
---@field updated_at string
---@field edit_count integer
---@field replies kenjutu.MaterializedReply[]

---@class kenjutu.MaterializedReply
---@field id string
---@field body string
---@field created_at string
---@field updated_at string
---@field edit_count integer

---@class kenjutu.FileComments
---@field file_path string
---@field comments kenjutu.PortedComment[]

---@class kenjutu.GetCommentsResult
---@field files kenjutu.FileComments[]

---@param dir string
---@param change_id string
---@param commit_id string
---@param cb fun(err: string|nil, result: kenjutu.GetCommentsResult|nil)
function M.get_comments(dir, change_id, commit_id, cb)
  send_request(dir, "get-comments", {
    change_id = change_id,
    commit = commit_id,
  }, cb)
end

---@class kenjutu.AddCommentOptions
---@field dir string
---@field change_id string
---@field commit_id string
---@field file_path string
---@field side "Old"|"New"
---@field line integer
---@field start_line integer|nil
---@field body string

---@param opts kenjutu.AddCommentOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.add_comment(opts, cb)
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

---@class kenjutu.ReplyToCommentOptions
---@field dir string
---@field change_id string
---@field file_path string
---@field parent_comment_id string
---@field body string

---@param opts kenjutu.ReplyToCommentOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.reply_to_comment(opts, cb)
  send_request(opts.dir, "reply-to-comment", {
    change_id = opts.change_id,
    file = opts.file_path,
    parent_comment_id = opts.parent_comment_id,
    body = opts.body,
  }, cb)
end

---@class kenjutu.EditCommentOptions
---@field dir string
---@field change_id string
---@field file_path string
---@field comment_id string
---@field body string

---@param opts kenjutu.EditCommentOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.edit_comment(opts, cb)
  send_request(opts.dir, "edit-comment", {
    change_id = opts.change_id,
    file = opts.file_path,
    comment_id = opts.comment_id,
    body = opts.body,
  }, cb)
end

---@class kenjutu.ResolveCommentOptions
---@field dir string
---@field change_id string
---@field file_path string
---@field comment_id string

---@param opts kenjutu.ResolveCommentOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.resolve_comment(opts, cb)
  send_request(opts.dir, "resolve-comment", {
    change_id = opts.change_id,
    file = opts.file_path,
    comment_id = opts.comment_id,
  }, cb)
end

---@param opts kenjutu.ResolveCommentOptions
---@param cb fun(err: string|nil, result: table|nil)
function M.unresolve_comment(opts, cb)
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
