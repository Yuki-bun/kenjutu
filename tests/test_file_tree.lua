local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local file_tree = require("kenjutu.file_tree")
local build_tree = file_tree._test.build_tree
local review_indicator = file_tree._test.review_indicator
local status_indicator = file_tree._test.status_indicator
local format_file_line = file_tree._test.format_file_line
local format_dir_line = file_tree._test.format_dir_line
local count_reviewed = file_tree._test.count_reviewed

---@param overrides table|nil
---@return kenjutu.FileEntry
local function make_file(overrides)
  return vim.tbl_extend("force", {
    newPath = "file.lua",
    oldPath = "file.lua",
    status = "modified",
    reviewStatus = "unreviewed",
    additions = 0,
    deletions = 0,
  }, overrides or {})
end

-- build_tree ------------------------------------------------------------------

T["build_tree"] = MiniTest.new_set()

T["build_tree"]["single file at root"] = function()
  local files = { make_file({ newPath = "foo.lua", oldPath = "foo.lua" }) }
  local tree = build_tree(files)
  expect.equality(#tree, 1)
  expect.equality(tree[1].type, "file")
  expect.equality(tree[1].name, "foo.lua")
end

T["build_tree"]["nested directory"] = function()
  local files = { make_file({ newPath = "a/b/c.lua", oldPath = "a/b/c.lua" }) }
  local tree = build_tree(files)
  -- Single-child compaction: a/b should compact into one dir node
  expect.equality(#tree, 1)
  expect.equality(tree[1].type, "directory")
  expect.equality(tree[1].name, "a/b")
  expect.equality(#tree[1].children, 1)
  expect.equality(tree[1].children[1].type, "file")
  expect.equality(tree[1].children[1].name, "c.lua")
end

T["build_tree"]["no compaction when dir has multiple children"] = function()
  local files = {
    make_file({ newPath = "src/a.lua", oldPath = "src/a.lua" }),
    make_file({ newPath = "src/b.lua", oldPath = "src/b.lua" }),
  }
  local tree = build_tree(files)
  expect.equality(#tree, 1)
  expect.equality(tree[1].type, "directory")
  expect.equality(tree[1].name, "src")
  expect.equality(#tree[1].children, 2)
end

T["build_tree"]["sorts directories before files"] = function()
  local files = {
    make_file({ newPath = "z.lua", oldPath = "z.lua" }),
    make_file({ newPath = "dir/a.lua", oldPath = "dir/a.lua" }),
  }
  local tree = build_tree(files)
  expect.equality(tree[1].type, "directory")
  expect.equality(tree[1].name, "dir")
  expect.equality(tree[2].type, "file")
  expect.equality(tree[2].name, "z.lua")
end

T["build_tree"]["sorts alphabetically within group"] = function()
  local files = {
    make_file({ newPath = "c.lua", oldPath = "c.lua" }),
    make_file({ newPath = "a.lua", oldPath = "a.lua" }),
    make_file({ newPath = "b.lua", oldPath = "b.lua" }),
  }
  local tree = build_tree(files)
  expect.equality(tree[1].name, "a.lua")
  expect.equality(tree[2].name, "b.lua")
  expect.equality(tree[3].name, "c.lua")
end

T["build_tree"]["deep single-child compaction"] = function()
  local files = { make_file({ newPath = "a/b/c/d/e.lua", oldPath = "a/b/c/d/e.lua" }) }
  local tree = build_tree(files)
  expect.equality(#tree, 1)
  expect.equality(tree[1].type, "directory")
  expect.equality(tree[1].name, "a/b/c/d")
  expect.equality(tree[1].children[1].name, "e.lua")
end

T["build_tree"]["compaction stops when dir has mixed children"] = function()
  local files = {
    make_file({ newPath = "a/b/x.lua", oldPath = "a/b/x.lua" }),
    make_file({ newPath = "a/b/c/y.lua", oldPath = "a/b/c/y.lua" }),
  }
  local tree = build_tree(files)
  -- "a" has one child "b" which has two children -> compact "a/b"
  expect.equality(#tree, 1)
  expect.equality(tree[1].name, "a/b")
  expect.equality(#tree[1].children, 2)
end

-- review_indicator ------------------------------------------------------------

T["review_indicator"] = MiniTest.new_set()

T["review_indicator"]["reviewed"] = function()
  local ind, hl = review_indicator("reviewed")
  expect.equality(ind, "[x]")
  expect.equality(hl, "KenjutuReviewed")
end

T["review_indicator"]["partiallyReviewed"] = function()
  local ind, hl = review_indicator("partiallyReviewed")
  expect.equality(ind, "[~]")
  expect.equality(hl, "KenjutuPartial")
end

T["review_indicator"]["reviewedReverted"] = function()
  local ind, hl = review_indicator("reviewedReverted")
  expect.equality(ind, "[!]")
  expect.equality(hl, "KenjutuReverted")
end

T["review_indicator"]["unreviewed"] = function()
  local ind, hl = review_indicator("unreviewed")
  expect.equality(ind, "[ ]")
  expect.equality(hl, nil)
end

-- status_indicator ------------------------------------------------------------

T["status_indicator"] = MiniTest.new_set()

T["status_indicator"]["maps known statuses"] = function()
  local cases = {
    { "added", "A", "KenjutuStatusA" },
    { "modified", "M", "KenjutuStatusM" },
    { "deleted", "D", "KenjutuStatusD" },
    { "renamed", "R", "KenjutuStatusR" },
    { "copied", "C", "KenjutuStatusC" },
    { "typechange", "T", "KenjutuStatusT" },
  }
  for _, case in ipairs(cases) do
    local letter, hl = status_indicator(case[1])
    expect.equality(letter, case[2])
    expect.equality(hl, case[3])
  end
end

T["status_indicator"]["unknown status returns ?"] = function()
  local letter, _ = status_indicator("whatever")
  expect.equality(letter, "?")
end

-- count_reviewed --------------------------------------------------------------

T["count_reviewed"] = MiniTest.new_set()

T["count_reviewed"]["counts only reviewed files"] = function()
  local files = {
    make_file({ reviewStatus = "reviewed" }),
    make_file({ reviewStatus = "unreviewed" }),
    make_file({ reviewStatus = "reviewed" }),
    make_file({ reviewStatus = "partiallyReviewed" }),
  }
  expect.equality(count_reviewed(files), 2)
end

T["count_reviewed"]["returns 0 for empty list"] = function()
  expect.equality(count_reviewed({}), 0)
end

-- format_file_line ------------------------------------------------------------

T["format_file_line"] = MiniTest.new_set()

T["format_file_line"]["produces expected text layout"] = function()
  local file = make_file({
    newPath = "src/foo.lua",
    reviewStatus = "reviewed",
    status = "modified",
    additions = 5,
    deletions = 3,
  })
  local result = format_file_line(file, "")
  expect.equality(type(result.text), "string")
  -- Should contain the indicator, filename, status, and stats
  expect.no_equality(result.text:find("%[x%]"), nil)
  expect.no_equality(result.text:find("foo.lua"), nil)
  expect.no_equality(result.text:find("M"), nil)
  expect.no_equality(result.text:find("+5"), nil)
  expect.no_equality(result.text:find("-3"), nil)
end

T["format_file_line"]["omits stats when zero"] = function()
  local file = make_file({ additions = 0, deletions = 0 })
  local result = format_file_line(file, "")
  expect.equality(result.text:find("+"), nil)
  expect.equality(result.text:find("-"), nil)
end

T["format_file_line"]["respects indent"] = function()
  local file = make_file()
  local result = format_file_line(file, "    ")
  expect.equality(result.text:sub(1, 4), "    ")
end

-- format_dir_line -------------------------------------------------------------

T["format_dir_line"] = MiniTest.new_set()

T["format_dir_line"]["produces directory line with highlight"] = function()
  local result = format_dir_line("src/components", "")
  expect.no_equality(result.text:find("src/components"), nil)
  expect.equality(#result.highlights, 1)
  expect.equality(result.highlights[1][3], "KenjutuDir")
end

return T
