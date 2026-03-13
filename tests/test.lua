---@diagnostic disable: duplicate-set-field
local M = {}

local total = 0
local failed = 0
local debug_messages = {}

---@param left any
---@param right any
---@param msg? string
function M.eq(left, right, msg)
  if not vim.deep_equal(left, right) then
    local error_msg = ""
    if msg then
      error_msg = msg .. "\n"
    end
    error_msg = error_msg .. "expected " .. vim.inspect(right) .. ", got " .. vim.inspect(left)
    error(error_msg, 2)
  end
end

---@param left any
---@param right any
---@param msg? string
function M.neq(left, right, msg)
  if vim.deep_equal(left, right) then
    local error_msg = ""
    if msg then
      error_msg = msg .. "\n"
    end
    error_msg = error_msg .. "expected values to differ, got " .. vim.inspect(left) .. " and " .. vim.inspect(right)
    error(error_msg, 2)
  end
end

----@param value any
----@param msg? string
function M.ok(value, msg)
  if not value then
    error(msg or ("expected truthy value, got " .. tostring(value)), 2)
  end
end

---@param fn function
function M.throws(fn)
  local ok = pcall(fn)
  if ok then
    error("expected function to error", 2)
  end
end

---@param ... any
function M.debug(...)
  local n = select("#", ...)
  local parts = {}
  for i = 1, n do
    table.insert(parts, vim.inspect((select(i, ...))))
  end
  table.insert(debug_messages, table.concat(parts, " "))
end

---@param name string
---@param fn function
function M.run_case(name, fn)
  total = total + 1
  debug_messages = {}
  local orig_notify = vim.notify
  local orig_echo = vim.api.nvim_echo
  ---@param msg string
  ---@param level integer|nil
  ---@param opts table|nil
  ---@diagnostic disable-next-line: unused-local
  vim.notify = function(msg, level, opts) end
  ---@param chunks any[]
  ---@param history boolean,
  ---@param opts vim.api.keyset.echo_opts
  ---@diagnostic disable-next-line: unused-local
  vim.api.nvim_echo = function(chunks, history, opts) end
  local ok, err = pcall(fn)
  vim.notify = orig_notify
  vim.api.nvim_echo = orig_echo
  if ok then
    io.write("  \027[32mPASS\027[0m  " .. name .. "\n")
  else
    failed = failed + 1
    io.write("  \027[31mFAIL\027[0m  " .. name .. "\n")
    for line in tostring(err):gmatch("[^\n]+") do
      io.write("        " .. line .. "\n")
    end
    for _, msg in ipairs(debug_messages) do
      for line in msg:gmatch("[^\n]+") do
        io.write("        \027[33mDEBUG\027[0m " .. line .. "\n")
      end
    end
  end
end

function M.set_file(path)
  io.write(path .. "\n")
end

function M.run()
  total = 0
  failed = 0

  local files = vim.fn.glob("tests/test_*.lua", false, true)
  table.sort(files)

  for _, file in ipairs(files) do
    M.set_file(file)
    dofile(file)
  end

  io.write("\n")
  if failed > 0 then
    io.write(string.format("\027[31m%d/%d failed\027[0m\n", failed, total))
    os.exit(1)
  else
    io.write(string.format("\027[32m%d passed\027[0m\n", total))
  end
end

return M
