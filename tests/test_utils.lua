local t = require("tests.test")

local utils = require("kenjutu.utils")

-- file_path -------------------------------------------------------------------

t.run_case("file_path returns newPath when present", function()
  local file = {
    newPath = "src/foo.lua",
    oldPath = "src/bar.lua",
    status = "renamed",
    reviewStatus = "unreviewed",
    additions = 0,
    deletions = 0,
  }
  t.eq(utils.file_path(file), "src/foo.lua")
end)

t.run_case("file_path returns oldPath when newPath is nil", function()
  local file =
    { oldPath = "src/bar.lua", status = "deleted", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  t.eq(utils.file_path(file), "src/bar.lua")
end)

t.run_case("file_path errors when both paths are nil", function()
  local file = { status = "deleted", reviewStatus = "unreviewed", additions = 0, deletions = 0 }
  t.throws(function()
    utils.file_path(file)
  end)
end)

-- await_all -------------------------------------------------------------------

t.run_case("await_all collects results from multiple tasks", function()
  local done = false
  local got_err, got_results
  utils.await_all({
    a = function(cb)
      vim.schedule(function()
        cb(nil, 1)
      end)
    end,
    b = function(cb)
      vim.schedule(function()
        cb(nil, 2)
      end)
    end,
  }, function(err, results)
    got_err, got_results = err, results
    done = true
  end)
  vim.wait(5000, function()
    return done
  end)
  t.eq(done, true)
  t.eq(got_err, nil)
  t.eq(got_results, { a = 1, b = 2 })
end)

t.run_case("await_all short-circuits on first error", function()
  local done = false
  local got_err, got_results
  utils.await_all({
    a = function(cb)
      vim.schedule(function()
        cb("boom", nil)
      end)
    end,
    b = function(cb)
      vim.schedule(function()
        cb(nil, 2)
      end)
    end,
  }, function(err, results)
    got_err, got_results = err, results
    done = true
  end)
  vim.wait(5000, function()
    return done
  end)
  t.eq(done, true)
  t.eq(got_err, "boom")
  t.eq(got_results, nil)
end)

t.run_case("await_all handles empty task table", function()
  local done = false
  local got_err, got_results
  utils.await_all({}, function(err, results)
    got_err, got_results = err, results
    done = true
  end)
  vim.wait(5000, function()
    return done
  end)
  t.eq(done, true)
  t.eq(got_err, nil)
  t.eq(got_results, {})
end)
