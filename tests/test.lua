local M = {}

local total = 0
local failed = 0

function M.eq(left, right)
  if not vim.deep_equal(left, right) then
    error("expected " .. vim.inspect(right) .. ", got " .. vim.inspect(left), 2)
  end
end

function M.neq(left, right)
  if vim.deep_equal(left, right) then
    error("expected values to differ, got " .. vim.inspect(left), 2)
  end
end

function M.ok(value, msg)
  if not value then
    error(msg or ("expected truthy value, got " .. tostring(value)), 2)
  end
end

function M.throws(fn)
  local ok = pcall(fn)
  if ok then
    error("expected function to error", 2)
  end
end

function M.run_case(name, fn)
  total = total + 1
  local ok, err = pcall(fn)
  if ok then
    io.write("  \027[32mPASS\027[0m  " .. name .. "\n")
  else
    failed = failed + 1
    io.write("  \027[31mFAIL\027[0m  " .. name .. "\n")
    for line in tostring(err):gmatch("[^\n]+") do
      io.write("        " .. line .. "\n")
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
