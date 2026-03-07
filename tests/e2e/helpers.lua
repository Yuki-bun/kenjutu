local M = {}

--- Run a shell command synchronously and return stdout.
---@param cmd string[]
---@param opts? {cwd?: string, stdin?: string}
---@return string stdout
---@return string stderr
---@return integer code
function M.shell(cmd, opts)
  opts = opts or {}
  local obj = vim.system(cmd, {
    text = true,
    cwd = opts.cwd,
    stdin = opts.stdin,
  }):wait(10000)
  return obj.stdout or "", obj.stderr or "", obj.code
end

--- Create a fresh jj+git repository in a temp directory.
---@return {path: string, cleanup: fun()}
function M.create_repo()
  local dir = vim.fn.tempname() .. "_kenjutu_e2e"
  vim.fn.mkdir(dir, "p")

  M.shell({ "git", "init", "-b", "main" }, { cwd = dir })
  M.shell({ "git", "config", "user.name", "Test User" }, { cwd = dir })
  M.shell({ "git", "config", "user.email", "test@test.com" }, { cwd = dir })

  -- Initial commit so jj has something to work with
  M.shell({ "git", "commit", "--allow-empty", "-m", "root" }, { cwd = dir })

  M.shell({ "jj", "git", "init", "--colocate" }, { cwd = dir })
  M.shell({ "jj", "config", "set", "--repo", "user.name", "Test User" }, { cwd = dir })
  M.shell({ "jj", "config", "set", "--repo", "user.email", "test@test.com" }, { cwd = dir })

  return {
    path = dir,
    cleanup = function()
      vim.fn.delete(dir, "rf")
    end,
  }
end

--- Write a file into the repo.
---@param repo {path: string}
---@param path string relative file path
---@param content string file content
function M.write_file(repo, path, content)
  local full = repo.path .. "/" .. path
  local parent = vim.fn.fnamemodify(full, ":h")
  if vim.fn.isdirectory(parent) == 0 then
    vim.fn.mkdir(parent, "p")
  end
  local f = io.open(full, "w")
  assert(f, "failed to open " .. full)
  f:write(content)
  f:close()
end

--- Delete a file from the repo.
---@param repo {path: string}
---@param path string relative file path
function M.delete_file(repo, path)
  vim.fn.delete(repo.path .. "/" .. path)
end

--- Run a jj command in the repo synchronously.
---@param repo {path: string}
---@param args string[]
---@return string stdout
function M.jj(repo, args)
  local cmd = { "jj" }
  for _, a in ipairs(args) do
    table.insert(cmd, a)
  end
  local stdout, stderr, code = M.shell(cmd, { cwd = repo.path })
  assert(code == 0, "jj failed: " .. stderr)
  return stdout
end

---@class kenjutu.e2e.CommitInfo
---@field change_id string
---@field commit_id string

--- Commit current working copy and return the created commit's IDs.
---@param repo {path: string}
---@param message string
---@return kenjutu.e2e.CommitInfo
function M.jj_commit(repo, message)
  M.jj(repo, { "commit", "-m", message })

  local stdout = M.jj(repo, {
    "log",
    "-r",
    "@-",
    "--no-graph",
    "--no-pager",
    "-T",
    'change_id ++ "\\x00" ++ commit_id',
  })
  local parts = vim.split(vim.trim(stdout), "\0", { plain = true })
  return {
    change_id = parts[1],
    commit_id = parts[2],
  }
end

--- Get the working copy's change_id and commit_id.
---@param repo {path: string}
---@return kenjutu.e2e.CommitInfo
function M.jj_working_copy(repo)
  local stdout = M.jj(repo, {
    "log",
    "-r",
    "@",
    "--no-graph",
    "--no-pager",
    "-T",
    'change_id ++ "\\x00" ++ commit_id',
  })
  local parts = vim.split(vim.trim(stdout), "\0", { plain = true })
  return {
    change_id = parts[1],
    commit_id = parts[2],
  }
end

--- Convert an async callback-style function into a synchronous call.
--- Runs the event loop until the callback fires or timeout is reached.
---@param fn fun(callback: fun(err: string|nil, result: any))
---@param timeout_ms? integer defaults to 10000
---@return string|nil err
---@return any result
function M.sync(fn, timeout_ms)
  timeout_ms = timeout_ms or 10000
  local done = false
  local out_err, out_result

  fn(function(err, result)
    out_err = err
    out_result = result
    done = true
  end)

  vim.wait(timeout_ms, function()
    return done
  end, 50)

  if not done then
    return "sync: timed out after " .. timeout_ms .. "ms", nil
  end

  return out_err, out_result
end

--- Wait until a predicate returns true, processing the event loop.
---@param predicate fun(): boolean
---@param timeout_ms? integer defaults to 10000
---@return boolean success
function M.wait_until(predicate, timeout_ms)
  timeout_ms = timeout_ms or 10000
  return vim.wait(timeout_ms, predicate, 50) ~= nil
end

--- Find a buffer by filetype in the current tab.
---@param ft string
---@return integer|nil bufnr
---@return integer|nil winnr
function M.find_buf_by_ft(ft)
  for _, w in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
    local b = vim.api.nvim_win_get_buf(w)
    if vim.bo[b].filetype == ft then
      return b, w
    end
  end
  return nil, nil
end

--- Get all lines from a buffer as a single string.
---@param bufnr integer
---@return string[]
function M.buf_lines(bufnr)
  return vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
end

--- Check if any line in the buffer contains the given pattern.
---@param bufnr integer
---@param pattern string Lua pattern
---@return boolean
function M.buf_contains(bufnr, pattern)
  for _, line in ipairs(M.buf_lines(bufnr)) do
    if line:find(pattern) then
      return true
    end
  end
  return false
end

return M
