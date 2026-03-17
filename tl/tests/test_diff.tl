local record TestModule
   eq: function(any, any, string)
   neq: function(any, any, string)
   ok: function(any, string)
   throws: function(function())
   run_case: function(string, function())
end

local t = require("tests.test") as TestModule

local record UtilsModule
   mock_all: function()
   restore_all: function()
   review_wins: function(): integer, integer, integer
end

local t_util = require("tests.utils") as UtilsModule

local kjn = require("kenjutu.kjn") as any
local review = require("kenjutu.review") as any

local mock_change_id = "zzzzzzzz"

local mock_content = {
  base = "base line1\nbase line2\nbase line3\n",
  marker = "marker line1\nmarker line2\nmarker line3\n",
  target = "target line1\ntarget line2\ntarget line3\n",
} as {string:string}

local base_lines: {string} = { "base line1", "base line2", "base line3" }
local marker_lines: {string} = { "marker line1", "marker line2", "marker line3" }
local target_lines: {string} = { "target line1", "target line2", "target line3" }

local function win_buf_lines(winnr: integer): {string}
  local bufnr = vim.api.nvim_win_get_buf(winnr)
  return vim.api.nvim_buf_get_lines(bufnr, 0, -1, false) as {string}
end

local function win_buf_name(winnr: integer): string
  local bufnr = vim.api.nvim_win_get_buf(winnr)
  return vim.api.nvim_buf_get_name(bufnr) as string
end

local function open_review(file_opts: {string:any}, blob_map: {string:string})
  file_opts = file_opts or {}
  blob_map = blob_map or mock_content

  local file = {
    newPath = "src/foo.lua",
    oldPath = "src/foo.lua",
    status = "modified",
    reviewStatus = file_opts.reviewStatus or "unreviewed",
    additions = 3,
    deletions = 1,
    isBinary = false,
  } as any

  kjn.files = function(_: any, _: any, cb: function(any, any))
    cb(nil, {
      files = { file },
      commitId = "abc123",
      changeId = mock_change_id,
    })
  end
  kjn.fetch_blob = function(opts: any, cb: function(any, string))
    cb(nil, blob_map[opts.tree_kind as string] or "")
  end

  local log_bufnr = vim.api.nvim_get_current_buf()
  local commit = { change_id = mock_change_id, commit_id = "abc123" } as any
  review.open(vim.fn.getcwd(), commit, log_bufnr, function() end)

  vim.api.nvim_feedkeys("jjj", "x", false)
  vim.cmd("doautocmd CursorMoved")
end

local function diff_case(name: string, fn: function())
  t_util.mock_all()
  t.run_case(name, function()
    vim.cmd("tabnew")
    local success: boolean, err: any = pcall(fn)
    while #(vim.api.nvim_list_tabpages() as {integer}) > 1 do
      vim.cmd("tabclose!")
    end
    if not success then
      error(err, 0)
    end
  end)
  t_util.restore_all()
end

diff_case("review.open produces three-pane layout with diff enabled", function()
  open_review()

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  t.ok((vim.wo[diff_left] as any).diff, "left diff window should have diff enabled")
  t.ok((vim.wo[diff_right] as any).diff, "right diff window should have diff enabled")
end)

diff_case("unreviewed file loads marker and target", function()
  open_review({ reviewStatus = "unreviewed" })

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  t.eq(win_buf_lines(diff_left), marker_lines)
  t.eq(win_buf_lines(diff_right), target_lines)
  t.ok(win_buf_name(diff_left):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(diff_right):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("reviewed file loads base and marker", function()
  open_review({ reviewStatus = "reviewed" })

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  t.eq(win_buf_lines(diff_left), base_lines)
  t.eq(win_buf_lines(diff_right), marker_lines)
  t.ok(win_buf_name(diff_left):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(diff_right):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from remaining to reviewed", function()
  open_review({ reviewStatus = "unreviewed" })

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)
  vim.api.nvim_feedkeys("t", "x", false)

  t.eq(win_buf_lines(diff_left), base_lines)
  t.eq(win_buf_lines(diff_right), marker_lines)
  t.ok(win_buf_name(diff_left):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(diff_right):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from reviewed to remaining", function()
  open_review({ reviewStatus = "reviewed" })

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)
  vim.api.nvim_feedkeys("t", "x", false)

  t.eq(win_buf_lines(diff_left), marker_lines)
  t.eq(win_buf_lines(diff_right), target_lines)
  t.ok(win_buf_name(diff_left):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(diff_right):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("toggle_mode round-trip preserves marker content", function()
  open_review({ reviewStatus = "unreviewed" })

  local _: integer, diff_left: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)

  vim.api.nvim_feedkeys("t", "x", false)
  t.eq(win_buf_lines(diff_right), marker_lines, "marker content should be preserved after toggle to reviewed")

  vim.api.nvim_feedkeys("t", "x", false)
  t.eq(win_buf_lines(diff_left), marker_lines, "marker content should be preserved after round-trip")
end)

diff_case("close restores single-window layout", function()
  open_review()

  local _: integer, _: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)
  vim.api.nvim_feedkeys("q", "x", false)

  local layout = vim.fn.winlayout() as {any}
  t.eq(layout[1], "leaf", "should have a single window after close")
end)

diff_case("fetch_blob error does not crash", function()
  open_review({}, {} as {string:string})
end)

diff_case("mark_action from non-marker buffer applies hunk via diffput", function()
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  local blob_map: {string:string} = { marker = left, target = right, base = "" }

  local got_content: string
  kjn.set_blob = function(_: any, content: string, cb: function(any))
    got_content = content
    cb(nil)
  end

  open_review({ reviewStatus = "unreviewed" }, blob_map)

  local _: integer, _: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)
  vim.api.nvim_win_set_cursor(diff_right, { 2, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action from marker buffer absorbs hunk via diffget", function()
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  local blob_map: {string:string} = { marker = left, target = right, base = "" }

  local got_content: string
  kjn.set_blob = function(_: any, content: string, cb: function(any))
    got_content = content
    cb(nil)
  end

  open_review({ reviewStatus = "unreviewed" }, blob_map)

  local _: integer, diff_left: integer, _: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_left)
  vim.api.nvim_win_set_cursor(diff_left, { 2, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action with visual selection applies only selected range", function()
  local left = "same1\nleft A\nsame2\nleft B\nsame3\n"
  local right = "same1\nright A\nsame2\nright B\nsame3\n"
  local blob_map: {string:string} = { marker = left, target = right, base = "" }

  local got_content: string
  kjn.set_blob = function(_: any, content: string, cb: function(any))
    got_content = content
    cb(nil)
  end

  open_review({ reviewStatus = "unreviewed" }, blob_map)

  local _: integer, _: integer, diff_right: integer = t_util.review_wins()
  vim.api.nvim_set_current_win(diff_right)
  vim.api.nvim_win_set_cursor(diff_right, { 2, 0 })
  vim.cmd("normal! V")
  vim.api.nvim_feedkeys("s", "x", false)

  t.eq(got_content, "same1\nright A\nsame2\nleft B\nsame3\n")
end)
