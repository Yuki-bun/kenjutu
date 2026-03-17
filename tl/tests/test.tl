local record M
   eq: function(any, any, string)
   neq: function(any, any, string)
   ok: function(any, string)
   throws: function(function())
   debug: function(...: any)
   run_case: function(string, function())
   set_file: function(string)
   run: function()
end

local total: integer = 0
local failed: integer = 0
local debug_messages: {string} = {}

function M.eq(left: any, right: any, msg: string)
  if not vim.deep_equal(left, right) then
    local error_msg = ""
    if msg then
      error_msg = msg .. "\n"
    end
    error_msg = error_msg .. "expected " .. vim.inspect(right) .. ", got " .. vim.inspect(left)
    error(error_msg, 2)
  end
end

function M.neq(left: any, right: any, msg: string)
  if vim.deep_equal(left, right) then
    local error_msg = ""
    if msg then
      error_msg = msg .. "\n"
    end
    error_msg = error_msg .. "expected values to differ, got " .. vim.inspect(left) .. " and " .. vim.inspect(right)
    error(error_msg, 2)
  end
end

function M.ok(value: any, msg: string)
  if not value then
    error(msg or ("expected truthy value, got " .. tostring(value)), 2)
  end
end

function M.throws(fn: function())
  local success: boolean = pcall(fn)
  if success then
    error("expected function to error", 2)
  end
end

function M.debug(...: any)
  local n = select("#", ...)
  local parts: {string} = {}
  for i = 1, n do
    table.insert(parts, vim.inspect((select(i, ...))))
  end
  table.insert(debug_messages, table.concat(parts, " "))
end

function M.run_case(name: string, fn: function())
  total = total + 1
  debug_messages = {}
  local orig_notify = vim.notify
  local orig_echo = vim.api.nvim_echo
  vim.notify = function(_: string, _: integer, _: {string:any}) end as any
  vim.api.nvim_echo = function(_: any, _: boolean, _: {string:any}) end as any
  local success: boolean, err: any = pcall(fn)
  vim.notify = orig_notify
  vim.api.nvim_echo = orig_echo
  if success then
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

function M.set_file(path: string)
  io.write(path .. "\n")
end

function M.run()
  total = 0
  failed = 0

  local files = vim.fn.glob("tests/test_*.lua", false, true) as {string}
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
