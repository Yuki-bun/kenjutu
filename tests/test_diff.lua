local t = require("tests.test")

local diff = require("kenjutu.diff")

local mock_content = {
  base = "base line1\nbase line2\nbase line3\n",
  marker = "marker line1\nmarker line2\nmarker line3\n",
  target = "target line1\ntarget line2\ntarget line3\n",
}

local base_lines = { "base line1", "base line2", "base line3" }
local marker_lines = { "marker line1", "marker line2", "marker line3" }
local target_lines = { "target line1", "target line2", "target line3" }

---@return fun(tree_kind: string, cb: fun(err: string|nil, content: string|nil))
local function make_loader()
  return function(tree_kind, cb)
    cb(nil, mock_content[tree_kind])
  end
end

---@return fun(tree_kind: string, cb: fun(err: string|nil, content: string|nil))
local function make_error_loader()
  return function(tree_kind, cb)
    cb("boom: " .. tree_kind, nil)
  end
end

local function make_file(review_status)
  return {
    newPath = "src/foo.lua",
    oldPath = "src/foo.lua",
    status = "modified",
    reviewStatus = review_status,
    additions = 3,
    deletions = 1,
    isBinary = false,
  }
end

---@param bufnr integer
---@return string[]
local function buf_lines(bufnr)
  return vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
end

---@param winnr integer
---@return string
local function win_buf_name(winnr)
  local bufnr = vim.api.nvim_win_get_buf(winnr)
  return vim.api.nvim_buf_get_name(bufnr)
end

local function diff_case(name, fn)
  t.run_case(name, function()
    vim.cmd("tabnew")
    local ok, err = pcall(fn)
    while #vim.api.nvim_list_tabpages() > 1 do
      vim.cmd("tabclose!")
    end
    if not ok then
      error(err, 0)
    end
  end)
end

diff_case("create produces a two-window diff layout", function()
  local anchor = vim.api.nvim_get_current_win()
  diff.create(anchor)

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.eq(#wins, 2)
  t.ok(vim.wo[wins[1]].diff, "left window should have diff enabled")
  t.ok(vim.wo[wins[2]].diff, "right window should have diff enabled")
end)

diff_case("set_file on unreviewed file loads marker and target", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  local loader = make_loader()

  state:set_file(make_file("unreviewed"), loader)

  t.eq(buf_lines(state.pane.left_bufnr), marker_lines)
  t.eq(buf_lines(state.pane.right_bufnr), target_lines)

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.ok(win_buf_name(wins[1]):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(wins[2]):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("set_file on reviewed file loads base and marker", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  local loader = make_loader()

  state:set_file(make_file("reviewed"), loader)

  t.eq(buf_lines(state.pane.left_bufnr), base_lines)
  t.eq(buf_lines(state.pane.right_bufnr), marker_lines)

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.ok(win_buf_name(wins[1]):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(wins[2]):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from remaining to reviewed", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  state:set_file(make_file("unreviewed"), make_loader())
  state:toggle_mode(make_loader())

  t.eq(buf_lines(state.pane.left_bufnr), base_lines)
  t.eq(buf_lines(state.pane.right_bufnr), marker_lines)

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.ok(win_buf_name(wins[1]):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(wins[2]):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from reviewed to remaining", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  state:set_file(make_file("reviewed"), make_loader())
  state:toggle_mode(make_loader())

  t.eq(buf_lines(state.pane.left_bufnr), marker_lines)
  t.eq(buf_lines(state.pane.right_bufnr), target_lines)

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.ok(win_buf_name(wins[1]):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(wins[2]):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("toggle_mode round-trip preserves shared buffer content", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  state:set_file(make_file("unreviewed"), make_loader())

  local wins = vim.api.nvim_tabpage_list_wins(0)
  local marker_before = buf_lines(vim.api.nvim_win_get_buf(wins[1]))

  state:toggle_mode(make_loader())
  local marker_after_toggle = buf_lines(vim.api.nvim_win_get_buf(wins[2]))
  t.eq(marker_before, marker_after_toggle, "marker content should be preserved after toggle to reviewed")

  state:toggle_mode(make_loader())
  local marker_after_roundtrip = buf_lines(vim.api.nvim_win_get_buf(wins[1]))
  t.eq(marker_before, marker_after_roundtrip, "marker content should be preserved after round-trip")
end)

diff_case("close leaves only the anchor window", function()
  local anchor = vim.api.nvim_get_current_win()
  local state = diff.create(anchor)

  t.eq(#vim.api.nvim_tabpage_list_wins(0), 2)

  state:close()

  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.eq(#wins, 1)
  t.eq(wins[1], anchor)
end)

diff_case("set_file loader error does not crash", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  local error_loader = make_error_loader()

  -- should not throw
  state:set_file(make_file("unreviewed"), error_loader)
end)

local function make_diffable_loader(left_content, right_content)
  return function(tree_kind, cb)
    if tree_kind == "marker" then
      cb(nil, left_content)
    elseif tree_kind == "target" then
      cb(nil, right_content)
    else
      cb(nil, "")
    end
  end
end

diff_case("mark_action from non-marker buffer applies hunk via diffput", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  vim.api.nvim_set_current_win(state.pane.right_winnr)
  vim.api.nvim_win_set_cursor(state.pane.right_winnr, { 2, 0 })

  local got_content
  state:mark_action(false, function(content)
    got_content = content
  end)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action from marker buffer absorbs hunk via diffget", function()
  local state = diff.create(vim.api.nvim_get_current_win())
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  vim.api.nvim_set_current_win(state.pane.left_winnr)
  vim.api.nvim_win_set_cursor(state.pane.left_winnr, { 2, 0 })

  local got_content
  state:mark_action(false, function(content)
    got_content = content
  end)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action with visual selection applies only selected range", function()
  local left = "same1\nleft A\nsame2\nleft B\nsame3\n"
  local right = "same1\nright A\nsame2\nright B\nsame3\n"

  local state = diff.create(vim.api.nvim_get_current_win())
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  vim.api.nvim_set_current_win(state.pane.right_winnr)
  vim.api.nvim_win_set_cursor(state.pane.right_winnr, { 2, 0 })

  vim.cmd("normal! V")

  local got_content
  state:mark_action(true, function(content)
    got_content = content
  end)

  t.eq(got_content, "same1\nright A\nsame2\nleft B\nsame3\n")
end)
