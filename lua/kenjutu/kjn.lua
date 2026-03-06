---@alias kenjutu.TreeKind "base" | "marker" | "target"

---@class kenjutu.FileEntry
---@field oldPath string|nil
---@field newPath string|nil
---@field status string "added"|"modified"|"deleted"|"renamed"|"copied"|"typechange"
---@field additions integer
---@field deletions integer
---@field isBinary boolean
---@field reviewStatus "reviewed"|"partiallyReviewed"|"unreviewed"|"reviewedReverted"

local M = {}

-- Resolve the kjn binary path relative to the plugin root.
-- This file lives at <plugin_root>/lua/kenjutu/kjn.lua, so going up 3
-- levels gives us the plugin root where target/release/kjn is built.
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

--- Build the full command table for a kjn invocation.
---@param dir string working directory
---@param args string[] subcommand + flags
---@return string[]
local function build_cmd(dir, args)
  local cmd = { kjn_bin, "--dir", dir }
  for _, arg in ipairs(args) do
    table.insert(cmd, arg)
  end
  return cmd
end

--- Extract a human-readable error message from a failed kjn invocation.
---@param stderr string
---@return string
local function parse_error(stderr)
  stderr = stderr or ""
  local ok, parsed = pcall(vim.fn.json_decode, vim.trim(stderr))
  if ok and type(parsed) == "table" and parsed.error then
    return parsed.error
  elseif stderr ~= "" then
    return vim.trim(stderr)
  end
  return "kjn failed"
end

--- Parse stdout as JSON, normalize vim.NIL values, and pass the result to callback.
---@param stdout string
---@param callback fun(err: string|nil, result: table|nil)
local function parse_json_output(stdout, callback)
  if stdout == "" then
    callback(nil, nil)
    return
  end

  local ok, result = pcall(vim.fn.json_decode, stdout)
  if not ok then
    callback("failed to parse kjn output: " .. tostring(result), nil)
    return
  end

  if type(result) == "table" then
    deep_convert_nil(result)
  end

  callback(nil, result)
end

--- Run a kjn subcommand asynchronously and return parsed JSON.
---@param dir string working directory
---@param args string[] subcommand + flags (e.g., {"files", "--commit", sha})
---@param callback fun(err: string|nil, result: table|nil)
function M.run(dir, args, callback)
  vim.system(
    build_cmd(dir, args),
    { text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        callback(parse_error(obj.stderr), nil)
        return
      end
      parse_json_output(obj.stdout or "", callback)
    end)
  )
end

--- Run a kjn subcommand asynchronously and return raw stdout.
--- Unlike `run`, this does NOT parse stdout as JSON.
---@param dir string working directory
---@param args string[] subcommand + flags
---@param callback fun(err: string|nil, stdout: string|nil)
function M.run_raw(dir, args, callback)
  vim.system(
    build_cmd(dir, args),
    { text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        callback(parse_error(obj.stderr), nil)
        return
      end
      callback(nil, obj.stdout or "")
    end)
  )
end

--- Run a kjn subcommand asynchronously, piping `stdin_content` to stdin,
--- and return parsed JSON.
---@param dir string working directory
---@param args string[] subcommand + flags
---@param stdin_content string content to pipe to stdin
---@param callback fun(err: string|nil, result: table|nil)
function M.run_with_stdin(dir, args, stdin_content, callback)
  vim.system(
    build_cmd(dir, args),
    { text = true, stdin = stdin_content },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        callback(parse_error(obj.stderr), nil)
        return
      end
      parse_json_output(obj.stdout or "", callback)
    end)
  )
end

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
  local args = {
    "blob",
    "--change-id",
    opts.change_id,
    "--commit",
    opts.commit_id,
    "--file",
    opts.file_path,
    "--tree",
    opts.tree_kind,
  }
  if opts.old_path and opts.old_path ~= opts.file_path then
    table.insert(args, "--old-path")
    table.insert(args, opts.old_path)
  end
  M.run_raw(opts.dir, args, cb)
end

return M
