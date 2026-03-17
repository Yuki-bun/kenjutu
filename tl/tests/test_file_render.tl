local record TestModule
   eq: function(any, any, string)
   neq: function(any, any, string)
   ok: function(any, string)
   throws: function(function())
   run_case: function(string, function())
end

local t = require("tests.test") as TestModule

local file_render = require("kenjutu.file_render") as any
local build_tree = file_render.build_tree as function(any): {any}

local function make_file(overrides: {string:any}): any
  return vim.tbl_extend("force", {
    newPath = "file.lua",
    oldPath = "file.lua",
    status = "modified",
    reviewStatus = "unreviewed",
    additions = 0,
    deletions = 0,
  }, overrides or {})
end

t.run_case("build_tree single file at root", function()
  local files = { make_file({ newPath = "foo.lua", oldPath = "foo.lua" }) }
  local tree = build_tree(files)
  t.eq(#tree, 1)
  t.eq((tree[1] as any).type, "file")
  t.eq((tree[1] as any).name, "foo.lua")
end)

t.run_case("build_tree nested directory", function()
  local files = { make_file({ newPath = "a/b/c.lua", oldPath = "a/b/c.lua" }) }
  local tree = build_tree(files)
  t.eq(#tree, 1)
  t.eq((tree[1] as any).type, "directory")
  t.eq((tree[1] as any).name, "a/b")
  t.eq(#((tree[1] as any).children as {any}), 1)
  t.eq(((tree[1] as any).children as {any})[1].type, "file")
  t.eq(((tree[1] as any).children as {any})[1].name, "c.lua")
end)

t.run_case("build_tree no compaction when dir has multiple children", function()
  local files = {
    make_file({ newPath = "src/a.lua", oldPath = "src/a.lua" }),
    make_file({ newPath = "src/b.lua", oldPath = "src/b.lua" }),
  }
  local tree = build_tree(files)
  t.eq(#tree, 1)
  t.eq((tree[1] as any).type, "directory")
  t.eq((tree[1] as any).name, "src")
  t.eq(#((tree[1] as any).children as {any}), 2)
end)

t.run_case("build_tree sorts directories before files", function()
  local files = {
    make_file({ newPath = "z.lua", oldPath = "z.lua" }),
    make_file({ newPath = "dir/a.lua", oldPath = "dir/a.lua" }),
  }
  local tree = build_tree(files)
  t.eq((tree[1] as any).type, "directory")
  t.eq((tree[1] as any).name, "dir")
  t.eq((tree[2] as any).type, "file")
  t.eq((tree[2] as any).name, "z.lua")
end)

t.run_case("build_tree sorts alphabetically within group", function()
  local files = {
    make_file({ newPath = "c.lua", oldPath = "c.lua" }),
    make_file({ newPath = "a.lua", oldPath = "a.lua" }),
    make_file({ newPath = "b.lua", oldPath = "b.lua" }),
  }
  local tree = build_tree(files)
  t.eq((tree[1] as any).name, "a.lua")
  t.eq((tree[2] as any).name, "b.lua")
  t.eq((tree[3] as any).name, "c.lua")
end)

t.run_case("build_tree deep single-child compaction", function()
  local files = { make_file({ newPath = "a/b/c/d/e.lua", oldPath = "a/b/c/d/e.lua" }) }
  local tree = build_tree(files)
  t.eq(#tree, 1)
  t.eq((tree[1] as any).type, "directory")
  t.eq((tree[1] as any).name, "a/b/c/d")
  t.eq(((tree[1] as any).children as {any})[1].name, "e.lua")
end)

t.run_case("build_tree compaction stops when dir has mixed children", function()
  local files = {
    make_file({ newPath = "a/b/x.lua", oldPath = "a/b/x.lua" }),
    make_file({ newPath = "a/b/c/y.lua", oldPath = "a/b/c/y.lua" }),
  }
  local tree = build_tree(files)
  t.eq(#tree, 1)
  t.eq((tree[1] as any).name, "a/b")
  t.eq(#((tree[1] as any).children as {any}), 2)
end)
