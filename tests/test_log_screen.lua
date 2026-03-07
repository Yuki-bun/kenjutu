---@diagnostic disable: duplicate-set-field
local t = require("tests.test")

local jj = require("kenjutu.jj")
local kjn = require("kenjutu.kjn")

local original_jj_log = jj.log
local original_jj_fetch_metadata = jj.fetch_commit_metadata
local original_jj_describe = jj.describe
local original_jj_new_commit = jj.new_commit
local original_kjn_files = kjn.files

local mock_log_result = {
  lines = {
    "o  abc123 user commit one",
    "  first commit message",
    "o  def456 user commit two",
    "  second commit message",
  },
  highlights = {},
  commits_by_line = {
    [1] = { change_id = "aaaa1111", commit_id = "cccc1111" },
    [3] = { change_id = "bbbb2222", commit_id = "dddd2222" },
  },
  commit_lines = { 1, 3 },
}

local function install_mocks()
  jj.log = function(_, callback)
    callback(nil, mock_log_result)
  end
  jj.fetch_commit_metadata = function(_, _, callback)
    callback(nil, { summary = "test", description = "", author = "test", timestamp = "1s ago" })
  end
  jj.describe = function(_, _, _, callback)
    callback(nil)
  end
  jj.new_commit = function(_, _, callback)
    callback(nil)
  end
  kjn.files = function(_, _, callback)
    callback(nil, { files = {}, changeId = "abc123", commitId = "abc123" })
  end
end

local function restore_mocks()
  jj.log = original_jj_log
  jj.fetch_commit_metadata = original_jj_fetch_metadata
  jj.describe = original_jj_describe
  jj.new_commit = original_jj_new_commit
  kjn.files = original_kjn_files
end

local function cleanup_tabs()
  while #vim.api.nvim_list_tabpages() > 1 do
    vim.cmd("tabclose!")
  end
end

---@param ft string
---@return integer|nil bufnr
---@return integer|nil winnr
local function find_buf_by_ft(ft)
  for _, w in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
    local b = vim.api.nvim_win_get_buf(w)
    if vim.bo[b].filetype == ft then
      return b, w
    end
  end
  return nil, nil
end

local function log_case(name, fn)
  t.run_case(name, function()
    install_mocks()
    local ok, err = pcall(fn)
    restore_mocks()
    cleanup_tabs()
    if not ok then
      error(err, 0)
    end
  end)
end

-- log screen ------------------------------------------------------------------

log_case("open creates tab with correct layout", function()
  local tabs_before = #vim.api.nvim_list_tabpages()
  require("kenjutu.log").open()

  t.eq(#vim.api.nvim_list_tabpages(), tabs_before + 1)
  t.neq(find_buf_by_ft("kenjutu-log"), nil)
  t.neq(find_buf_by_ft("kenjutu-log-files"), nil)
  t.eq(#vim.api.nvim_tabpage_list_wins(0), 2)
end)

log_case("j moves cursor to next commit line", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("j", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(winnr)[1], 3, "j should jump to second commit line")

  vim.api.nvim_feedkeys("j", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(winnr)[1], 3, "j at last commit should stay put")
end)

log_case("k moves cursor to previous commit line", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("k", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(winnr)[1], 1, "k should jump to first commit line")

  vim.api.nvim_feedkeys("k", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(winnr)[1], 1, "k at first commit should stay put")
end)

log_case("<CR> opens review screen for commit under cursor", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq(vim.bo.filetype, "kenjutu-review-files", "<CR> on commit line should open review screen")
end)

log_case("<CR> does nothing on non-commit line", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 2, 0 })
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq(vim.bo.filetype, "kenjutu-log", "<CR> on non-commit line should stay on log")
end)

log_case("r refreshes the log buffer content", function()
  require("kenjutu.log").open()
  local log_bufnr, winnr = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log buffer")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })

  local updated_lines = {
    "o  fff789 user commit three",
    "  third commit message",
    "o  abc123 user commit one",
    "  first commit message",
    "o  def456 user commit two",
    "  second commit message",
  }
  jj.log = function(_, callback)
    callback(nil, {
      lines = updated_lines,
      highlights = {},
      commits_by_line = {
        [1] = { change_id = "cccc3333", commit_id = "eeee3333" },
        [3] = { change_id = "aaaa1111", commit_id = "cccc1111" },
        [5] = { change_id = "bbbb2222", commit_id = "dddd2222" },
      },
      commit_lines = { 1, 3, 5 },
    })
  end

  vim.api.nvim_feedkeys("r", "x", false)

  local buf_lines = vim.api.nvim_buf_get_lines(log_bufnr, 0, -1, false)
  t.eq(buf_lines[1], updated_lines[1], "buffer content should reflect refreshed data")
  t.eq(vim.api.nvim_win_get_cursor(winnr)[1], 5, "cursor should follow the same commit after refresh")
end)

log_case("q closes the tab", function()
  require("kenjutu.log").open()
  local tabs_before = #vim.api.nvim_list_tabpages()

  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log buffer window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(#vim.api.nvim_list_tabpages(), tabs_before - 1)
end)

-- describe --------------------------------------------------------------------

log_case("d opens describe split with current description", function()
  jj.fetch_commit_metadata = function(_, _, callback)
    callback(nil, { summary = "fix: typo", description = "body line", author = "me", timestamp = "1s ago" })
  end

  require("kenjutu.log").open()
  local _, log_winnr = find_buf_by_ft("kenjutu-log")
  assert(log_winnr, "could not find log window")
  vim.api.nvim_set_current_win(log_winnr)
  vim.api.nvim_win_set_cursor(log_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "describe split should open")

  local desc_winnr = vim.api.nvim_get_current_win()
  local desc_height = vim.api.nvim_win_get_height(desc_winnr)
  local log_height = vim.api.nvim_win_get_height(log_winnr)
  t.eq(desc_height, log_height, "describe split should have same height as log")

  local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false)
  t.eq(lines, { "fix: typo", "body line" }, "buffer should contain the full description")
end)

log_case(":w in describe split calls jj describe and refreshes log", function()
  local captured_change_id = nil
  local captured_message = nil

  jj.fetch_commit_metadata = function(_, _, callback)
    callback(nil, { summary = "old msg", description = "", author = "me", timestamp = "1s ago" })
  end
  jj.describe = function(_, change_id, message, callback)
    captured_change_id = change_id
    captured_message = message
    callback(nil)
  end

  local updated_log_result = {
    lines = {
      "o  abc123 user commit one",
      "  new description",
      "o  def456 user commit two",
      "  second commit message",
    },
    highlights = {},
    commits_by_line = {
      [1] = { change_id = "aaaa1111", commit_id = "cccc1111" },
      [3] = { change_id = "bbbb2222", commit_id = "dddd2222" },
    },
    commit_lines = { 1, 3 },
  }

  require("kenjutu.log").open()
  local log_bufnr, winnr = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "describe split should be open")

  jj.log = function(_, callback)
    callback(nil, updated_log_result)
  end

  vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, { "new description" })
  vim.api.nvim_set_current_buf(desc_bufnr)
  vim.cmd("write")

  t.eq(captured_change_id, "aaaa1111", "should describe the correct commit")
  t.eq(captured_message, "new description", "should pass the edited message")
  t.eq(find_buf_by_ft("jjdescription"), nil, "describe split should close after save")

  local buf_lines = vim.api.nvim_buf_get_lines(log_bufnr, 0, -1, false)
  t.eq(buf_lines[2], "  new description", "log should show updated content after refresh")
end)

log_case("q in describe split closes without saving", function()
  local describe_called = false
  jj.describe = function(_, _, _, callback)
    describe_called = true
    callback(nil)
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr, desc_winnr = find_buf_by_ft("jjdescription")
  assert(desc_bufnr and desc_winnr, "describe split should be open")

  vim.api.nvim_set_current_win(desc_winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(describe_called, false, "jj describe should not be called on q")
  t.eq(find_buf_by_ft("jjdescription"), nil, "describe buffer should be gone")
end)

-- new commit ------------------------------------------------------------------

log_case("n creates new commit after cursor commit and refreshes log", function()
  local captured_change_id = nil
  jj.new_commit = function(_, change_id, callback)
    captured_change_id = change_id
    callback(nil)
  end

  local refreshed = false
  local updated_log_result = {
    lines = {
      "o  new111 user new commit",
      "  (no description set)",
      "o  abc123 user commit one",
      "  first commit message",
      "o  def456 user commit two",
      "  second commit message",
    },
    highlights = {},
    commits_by_line = {
      [1] = { change_id = "neww1111", commit_id = "ffff1111" },
      [3] = { change_id = "aaaa1111", commit_id = "cccc1111" },
      [5] = { change_id = "bbbb2222", commit_id = "dddd2222" },
    },
    commit_lines = { 1, 3, 5 },
  }
  jj.log = function(_, callback)
    refreshed = true
    callback(nil, updated_log_result)
  end

  require("kenjutu.log").open()
  local log_bufnr, winnr = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("n", "x", false)

  t.eq(captured_change_id, "aaaa1111", "should create new commit after the cursor commit")
  t.eq(refreshed, true, "log should refresh after creating new commit")
end)

log_case("n does nothing on non-commit line", function()
  local new_commit_called = false
  jj.new_commit = function(_, _, callback)
    new_commit_called = true
    callback(nil)
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 2, 0 })

  vim.api.nvim_feedkeys("n", "x", false)

  t.eq(new_commit_called, false, "jj new should not be called on non-commit line")
end)
