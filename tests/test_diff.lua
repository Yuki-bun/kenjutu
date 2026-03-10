local t = require("tests.test")

local diff = require("kenjutu.diff")

local mock_change_id = "zzzzzzzz"

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

--- Extract left and right window IDs from winlayout().
--- Expects a {"row", {{"leaf", L}, {"leaf", R}}} shape.
---@return integer left_winnr, integer right_winnr
local function diff_wins()
  local layout = vim.fn.winlayout()
  assert(layout[1] == "row", "expected row layout, got " .. layout[1])
  local children = layout[2]
  assert(#children == 2, "expected 2 children, got " .. #children)
  local left_winnr = children[1][2]
  local right_winnr = children[2][2]
  assert(type(left_winnr) == "number", "expected left winnr to be a number, got " .. type(left_winnr))
  assert(type(right_winnr) == "number", "expected right winnr to be a number, got " .. type(right_winnr))
  return left_winnr, right_winnr
end

---@param winnr integer
---@return string[]
local function win_buf_lines(winnr)
  local bufnr = vim.api.nvim_win_get_buf(winnr)
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
  diff.create(anchor, mock_change_id)

  local left, right = diff_wins()
  t.ok(vim.wo[left].diff, "left window should have diff enabled")
  t.ok(vim.wo[right].diff, "right window should have diff enabled")
end)

diff_case("set_file on unreviewed file loads marker and target", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("unreviewed"), make_loader())

  local left, right = diff_wins()
  t.eq(win_buf_lines(left), marker_lines)
  t.eq(win_buf_lines(right), target_lines)
  t.ok(win_buf_name(left):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(right):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("set_file on reviewed file loads base and marker", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("reviewed"), make_loader())

  local left, right = diff_wins()
  t.eq(win_buf_lines(left), base_lines)
  t.eq(win_buf_lines(right), marker_lines)
  t.ok(win_buf_name(left):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(right):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from remaining to reviewed", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("unreviewed"), make_loader())
  state:toggle_mode(make_loader())

  local left, right = diff_wins()
  t.eq(win_buf_lines(left), base_lines)
  t.eq(win_buf_lines(right), marker_lines)
  t.ok(win_buf_name(left):find(":base$") ~= nil, "left buffer name should end with :base")
  t.ok(win_buf_name(right):find(":marker$") ~= nil, "right buffer name should end with :marker")
end)

diff_case("toggle_mode from reviewed to remaining", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("reviewed"), make_loader())
  state:toggle_mode(make_loader())

  local left, right = diff_wins()
  t.eq(win_buf_lines(left), marker_lines)
  t.eq(win_buf_lines(right), target_lines)
  t.ok(win_buf_name(left):find(":marker$") ~= nil, "left buffer name should end with :marker")
  t.ok(win_buf_name(right):find(":target$") ~= nil, "right buffer name should end with :target")
end)

diff_case("toggle_mode round-trip preserves marker content", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("unreviewed"), make_loader())

  local left, right = diff_wins()

  state:toggle_mode(make_loader())
  t.eq(win_buf_lines(right), marker_lines, "marker content should be preserved after toggle to reviewed")

  state:toggle_mode(make_loader())
  t.eq(win_buf_lines(left), marker_lines, "marker content should be preserved after round-trip")
end)

diff_case("close leaves only the anchor window", function()
  local anchor = vim.api.nvim_get_current_win()
  local state = diff.create(anchor, mock_change_id)

  local _ = diff_wins() -- verify two-pane layout exists

  state:close()

  local layout = vim.fn.winlayout()
  t.eq(layout[1], "leaf", "should have a single window after close")
  t.eq(layout[2], anchor)
end)

diff_case("set_file loader error does not crash", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)

  -- should not throw
  state:set_file(make_file("unreviewed"), make_error_loader())
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
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  local _, right_winnr = diff_wins()
  vim.api.nvim_set_current_win(right_winnr)
  vim.api.nvim_win_set_cursor(right_winnr, { 2, 0 })

  local got_content
  state:mark_action(false, function(content)
    got_content = content
  end)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action from marker buffer absorbs hunk via diffget", function()
  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  local left = "same line1\nleft only\nsame line3\n"
  local right = "same line1\nright only\nsame line3\n"
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  local left_winnr, _ = diff_wins()
  vim.api.nvim_set_current_win(left_winnr)
  vim.api.nvim_win_set_cursor(left_winnr, { 2, 0 })

  local got_content
  state:mark_action(false, function(content)
    got_content = content
  end)

  t.eq(got_content, "same line1\nright only\nsame line3\n")
end)

diff_case("mark_action with visual selection applies only selected range", function()
  local left = "same1\nleft A\nsame2\nleft B\nsame3\n"
  local right = "same1\nright A\nsame2\nright B\nsame3\n"

  local state = diff.create(vim.api.nvim_get_current_win(), mock_change_id)
  state:set_file(make_file("unreviewed"), make_diffable_loader(left, right))

  local _, right_winnr = diff_wins()
  vim.api.nvim_set_current_win(right_winnr)
  vim.api.nvim_win_set_cursor(right_winnr, { 2, 0 })

  vim.cmd("normal! V")

  local got_content
  state:mark_action(true, function(content)
    got_content = content
  end)

  t.eq(got_content, "same1\nright A\nsame2\nleft B\nsame3\n")
end)
