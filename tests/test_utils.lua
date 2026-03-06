local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local utils = require("kenjutu.utils")

T["file_path"] = MiniTest.new_set()

T["file_path"]["returns newPath when present"] = function()
  local file = { newPath = "src/foo.lua", oldPath = "src/bar.lua", status = "renamed", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  expect.equality(utils.file_path(file), "src/foo.lua")
end

T["file_path"]["returns oldPath when newPath is nil"] = function()
  local file = { oldPath = "src/bar.lua", status = "deleted", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  expect.equality(utils.file_path(file), "src/bar.lua")
end

return T
