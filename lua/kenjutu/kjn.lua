---@alias kenjutu.TreeKind "base" | "marker" | "target"

---@class kenjutu.Kjn
local M = {}

local plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
local kjn_bin = plugin_dir .. "/target/release/kjn"

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

  daemon.job_id = vim.fn.jobstart({ kjn_bin, "--dir", dir, "serve" }, {
    on_stdout = function(_, data, _)
      for _, chunk in ipairs(data) do
        if chunk ~= "" then
          daemon.buf = daemon.buf .. chunk
          local newline_pos = daemon.buf:find("\n")
          while newline_pos do
            local line = daemon.buf:sub(1, newline_pos - 1)
            daemon.buf = daemon.buf:sub(newline_pos + 1)
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
            newline_pos = daemon.buf:find("\n")
          end
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
      daemons[dir] = nil
    end,
    stdout_buffered = false,
    stderr_buffered = false,
  })

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
    if not result or not result.content or result.content == "" then
      cb(nil, "")
      return
    end
    local decoded = vim.base64.decode(result.content)
    cb(nil, decoded)
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
  local encoded = vim.base64.encode(content)
  send_request(opts.dir, "set-blob", {
    change_id = opts.change_id,
    commit = opts.commit_id,
    file = opts.file_path,
    content = encoded,
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
