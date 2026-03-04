local M = {}

---@class kenjutu.Commit
---@field change_id string Full 32-char jj change identifier
---@field commit_id string Full git commit SHA

---@class kenjutu.LogResult
---@field lines string[] Display lines for the buffer (1-indexed)
---@field commits_by_line table<integer, kenjutu.Commit> Maps line number to commit data (commit lines only)
---@field commit_lines integer[] Sorted list of line numbers that are commit lines

local TEMPLATE = table.concat({
  '"\\x01"',
  "change_id",
  '"\\x00"',
  "commit_id",
  '"\\x00"',
  "description.first_line()",
  '"\\x00"',
  "author.name()",
  '"\\x00"',
  "author.timestamp().ago()",
  '"\\x00"',
  "immutable",
  '"\\x00"',
  "current_working_copy",
}, " ++ ") .. ' ++ "\\n"'

local REVSET = "mutable() | ancestors(mutable(), 2)"

--- Parse a single commit line (after splitting at \x01).
--- Fields are \x00-separated: change_id, commit_id, summary, author, timestamp, immutable, working_copy
---@param gutter string
---@param data string
---@return string display_line
---@return kenjutu.Commit commit_data
local function parse_commit_line(gutter, data)
  local fields = vim.split(data, "\0", { plain = true })
  local change_id = fields[1] or ""
  local commit_id = fields[2] or ""
  local summary = fields[3] or ""
  local author = fields[4] or ""
  local timestamp = fields[5] or ""

  local change_id_short = change_id:sub(1, 8)

  local parts = { gutter .. change_id_short }
  if summary ~= "" then
    table.insert(parts, summary)
  end
  if author ~= "" then
    table.insert(parts, author)
  end
  if timestamp ~= "" then
    table.insert(parts, timestamp)
  end

  local display_line = table.concat(parts, "  ")
  local commit_data = {
    change_id = change_id,
    commit_id = commit_id,
  }

  return display_line, commit_data
end

--- Run `jj log` asynchronously and parse the output.
---@param dir string working directory
---@param callback fun(err: string|nil, result: kenjutu.LogResult|nil)
function M.log(dir, callback)
  vim.system(
    { "jj", "log", "--color", "never", "--no-pager", "-r", REVSET, "-T", TEMPLATE },
    { cwd = dir, text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        local err = obj.stderr or "jj log failed"
        callback(vim.trim(err), nil)
        return
      end

      local stdout = obj.stdout or ""
      local raw_lines = vim.split(stdout, "\n", { plain = true })
      local lines = {}
      local commits_by_line = {}
      local commit_lines = {}

      for _, raw in ipairs(raw_lines) do
        local marker_pos = raw:find("\x01", 1, true)
        if marker_pos then
          local gutter = raw:sub(1, marker_pos - 1)
          local data = raw:sub(marker_pos + 1)
          local display_line, commit_data = parse_commit_line(gutter, data)
          table.insert(lines, display_line)
          commits_by_line[#lines] = commit_data
          table.insert(commit_lines, #lines)
        elseif vim.trim(raw) ~= "" then
          table.insert(lines, raw)
        end
      end

      callback(nil, { lines = lines, commits_by_line = commits_by_line, commit_lines = commit_lines })
    end)
  )
end

return M
