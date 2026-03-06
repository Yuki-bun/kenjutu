local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local utils = require("kenjutu.utils")

-- file_path -------------------------------------------------------------------

T["file_path"] = MiniTest.new_set()

T["file_path"]["returns newPath when present"] = function()
  local file = {
    newPath = "src/foo.lua",
    oldPath = "src/bar.lua",
    status = "renamed",
    reviewStatus = "unreviewed",
    additions = 0,
    deletions = 0,
  }
  expect.equality(utils.file_path(file), "src/foo.lua")
end

T["file_path"]["returns oldPath when newPath is nil"] = function()
  local file =
    { oldPath = "src/bar.lua", status = "deleted", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  expect.equality(utils.file_path(file), "src/bar.lua")
end

T["file_path"]["errors when both paths are nil"] = function()
  local file = { status = "deleted", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  expect.error(function()
    utils.file_path(file)
  end)
end

-- await_all -------------------------------------------------------------------

T["await_all"] = MiniTest.new_set()

T["await_all"]["collects results from multiple tasks"] = function()
  local results_out
  utils.await_all({
    a = function(cb)
      cb(nil, 1)
    end,
    b = function(cb)
      cb(nil, 2)
    end,
  }, function(err, results)
    expect.equality(err, nil)
    results_out = results
  end)
  expect.equality(results_out.a, 1)
  expect.equality(results_out.b, 2)
end

T["await_all"]["short-circuits on first error"] = function()
  local got_err
  utils.await_all({
    a = function(cb)
      cb("boom", nil)
    end,
    b = function(cb)
      cb(nil, 2)
    end,
  }, function(err, results)
    got_err = err
    expect.equality(results, nil)
  end)
  expect.equality(got_err, "boom")
end

T["await_all"]["handles empty task table"] = function()
  local got_results
  utils.await_all({}, function(err, results)
    expect.equality(err, nil)
    got_results = results
  end)
  expect.equality(got_results, {})
end

return T
