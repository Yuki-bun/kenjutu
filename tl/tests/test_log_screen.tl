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
end

local t_util = require("tests.utils") as UtilsModule

local jj = require("kenjutu.jj") as any
local kjn = require("kenjutu.kjn") as any

local mock_log_result = {
  lines = {
    "o  aaaa user commit one",
    "  first commit message",
    "o  bbbb user commit two",
    "  second commit message",
  },
  highlights = {},
  commits_by_line = {
    [1] = { change_id = "aaaa1111", commit_id = "cccc1111" },
    [3] = { change_id = "bbbb2222", commit_id = "dddd2222" },
  },
  commit_lines = { 1, 3 },
} as any

local function install_mocks()
  t_util.mock_all()
  jj.log = function(_: any, callback: function(any, any))
    callback(nil, mock_log_result)
  end
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "test", description = "", author = "test", timestamp = "1s ago" })
  end
end

local function cleanup_tabs()
  while #(vim.api.nvim_list_tabpages() as {integer}) > 1 do
    vim.cmd("tabclose!")
  end
end

local function find_buf_by_ft(ft: string): integer, integer
  for _, w in ipairs(vim.api.nvim_tabpage_list_wins(0) as {integer}) do
    local b = vim.api.nvim_win_get_buf(w)
    if (vim.bo[b] as any).filetype == ft then
      return b, w
    end
  end
  return nil, nil
end

local function log_case(name: string, fn: function())
  t.run_case(name, function()
    install_mocks()
    local success: boolean, err: any = pcall(fn)
    t_util.restore_all()
    cleanup_tabs()
    if not success then
      error(err, 0)
    end
  end)
end

log_case("open creates tab with correct layout", function()
  local tabs_before = #(vim.api.nvim_list_tabpages() as {integer})
  require("kenjutu.log").open()

  t.eq(#(vim.api.nvim_list_tabpages() as {integer}), tabs_before + 1)
  t.neq(find_buf_by_ft("kenjutu-log"), nil)
  t.neq(find_buf_by_ft("kenjutu-log-files"), nil)
  t.eq(#(vim.api.nvim_tabpage_list_wins(0) as {integer}), 2)
end)

log_case("j moves cursor to next commit line", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("j", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(winnr) as {integer})[1], 3, "j should jump to second commit line")

  vim.api.nvim_feedkeys("j", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(winnr) as {integer})[1], 3, "j at last commit should stay put")
end)

log_case("k moves cursor to previous commit line", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("k", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(winnr) as {integer})[1], 1, "k should jump to first commit line")

  vim.api.nvim_feedkeys("k", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(winnr) as {integer})[1], 1, "k at first commit should stay put")
end)

log_case("<CR> opens review screen for commit under cursor", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq((vim.bo as any).filetype, "kenjutu-review-files", "<CR> on commit line should open review screen")
end)

log_case("<CR> does nothing on non-commit line", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 2, 0 })
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq((vim.bo as any).filetype, "kenjutu-log", "<CR> on non-commit line should stay on log")
end)

log_case("r refreshes the log buffer content", function()
  require("kenjutu.log").open()
  local log_bufnr: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log buffer")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })

  local updated_lines: {string} = {
    "o  ffff user commit three",
    "  third commit message",
    "o  aaaa user commit one",
    "  first commit message",
    "o  bbbb user commit two",
    "  second commit message",
  }
  jj.log = function(_: any, callback: function(any, any))
    callback(nil, {
      lines = updated_lines,
      highlights = {},
      commits_by_line = {
        [1] = { change_id = "ffff3333", commit_id = "eeee3333" },
        [3] = { change_id = "aaaa1111", commit_id = "cccc1111" },
        [5] = { change_id = "bbbb2222", commit_id = "dddd9999" },
      },
      commit_lines = { 1, 3, 5 },
    })
  end

  local updated_files = {
    { newPath = "src/new_file.lua", status = "added", reviewed = false, additions = 1, deletions = 0 },
  } as {any}
  kjn.files = function(_: any, _: any, callback: function(any, any))
    callback(nil, { files = updated_files, changeId = "bbbb2222", commitId = "dddd2222" })
  end

  vim.api.nvim_feedkeys("r", "x", false)

  local buf_lines = vim.api.nvim_buf_get_lines(log_bufnr, 0, -1, false) as {string}
  t.eq(buf_lines[1], updated_lines[1], "buffer content should reflect refreshed data")
  t.eq((vim.api.nvim_win_get_cursor(winnr) as {integer})[1], 5, "cursor should follow the same commit after refresh")

  local files_bufnr: integer = find_buf_by_ft("kenjutu-log-files")
  assert(files_bufnr, "file tree buffer should exist")
  local files_lines = vim.api.nvim_buf_get_lines(files_bufnr, 0, -1, false) as {string}
  local has_new_file = false
  for _, line in ipairs(files_lines) do
    if line:find("new_file.lua") then
      has_new_file = true
      break
    end
  end
  t.eq(has_new_file, true, "file tree should show updated files after refresh")
end)

log_case("q closes the tab", function()
  require("kenjutu.log").open()
  local tabs_before = #(vim.api.nvim_list_tabpages() as {integer})

  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log buffer window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(#(vim.api.nvim_list_tabpages() as {integer}), tabs_before - 1)
end)

log_case("d opens describe split with current description", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "fix: typo", description = "body line", author = "me", timestamp = "1s ago" })
  end

  require("kenjutu.log").open()
  local _: integer, log_winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(log_winnr, "could not find log window")
  vim.api.nvim_set_current_win(log_winnr)
  vim.api.nvim_win_set_cursor(log_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "describe split should open")

  local desc_winnr = vim.api.nvim_get_current_win()
  local desc_height = vim.api.nvim_win_get_height(desc_winnr)
  local log_height = vim.api.nvim_win_get_height(log_winnr)
  t.eq(desc_height, log_height, "describe split should have same height as log")

  local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false) as {string}
  t.eq(lines, { "fix: typo", "body line" }, "buffer should contain the full description")
end)

log_case(":w in describe split calls jj describe and refreshes log", function()
  local captured_change_id: string = nil
  local captured_message: string = nil

  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "old msg", description = "", author = "me", timestamp = "1s ago" })
  end
  jj.describe = function(_: any, change_id: string, message: string, callback: function(any))
    captured_change_id = change_id
    captured_message = message
    callback(nil)
  end

  local updated_log_result = {
    lines = {
      "o  aaaa user commit one",
      "  new description",
      "o  bbbb user commit two",
      "  second commit message",
    },
    highlights = {},
    commits_by_line = {
      [1] = { change_id = "aaaa1111", commit_id = "cccc1111" },
      [3] = { change_id = "bbbb2222", commit_id = "dddd2222" },
    },
    commit_lines = { 1, 3 },
  } as any

  require("kenjutu.log").open()
  local log_bufnr: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "describe split should be open")

  jj.log = function(_: any, callback: function(any, any))
    callback(nil, updated_log_result)
  end

  vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, { "new description" })
  vim.api.nvim_set_current_buf(desc_bufnr)
  vim.cmd("write")

  t.eq(captured_change_id, "aaaa1111", "should describe the correct commit")
  t.eq(captured_message, "new description", "should pass the edited message")
  t.eq(find_buf_by_ft("jjdescription"), nil, "describe split should close after save")

  local buf_lines = vim.api.nvim_buf_get_lines(log_bufnr, 0, -1, false) as {string}
  t.eq(buf_lines[2], "  new description", "log should show updated content after refresh")
end)

log_case("q in describe split closes without saving", function()
  local describe_called = false
  jj.describe = function(_: any, _: any, _: any, callback: function(any))
    describe_called = true
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("d", "x", false)

  local desc_bufnr: integer, desc_winnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_bufnr and desc_winnr, "describe split should be open")

  vim.api.nvim_set_current_win(desc_winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(describe_called, false, "jj describe should not be called on q")
  t.eq(find_buf_by_ft("jjdescription"), nil, "describe buffer should be gone")
end)

log_case("n creates new commit after cursor commit and refreshes log", function()
  local captured_change_id: string = nil
  jj.new_commit = function(_: any, change_id: string, callback: function(any))
    captured_change_id = change_id
    callback(nil)
  end

  require("kenjutu.log").open()

  local updated_lines: {string} = {
    "o  nnnn user new commit",
    "  (no description set)",
    "o  aaaa user commit one",
    "  first commit message",
    "o  bbbb user commit two",
    "  second commit message",
  }
  jj.log = function(_: any, callback: function(any, any))
    callback(nil, {
      lines = updated_lines,
      highlights = {},
      commits_by_line = {
        [1] = { change_id = "nnnn4444", commit_id = "ffff1111" },
        [3] = { change_id = "aaaa1111", commit_id = "cccc1111" },
        [5] = { change_id = "bbbb2222", commit_id = "dddd2222" },
      },
      commit_lines = { 1, 3, 5 },
    })
  end
  local log_bufnr: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(log_bufnr and winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("n", "x", false)

  t.eq(captured_change_id, "aaaa1111", "should create new commit after the cursor commit")
  local buf_lines = vim.api.nvim_buf_get_lines(log_bufnr, 0, -1, false) as {string}
  t.eq(buf_lines[1], updated_lines[1], "buffer should show refreshed log with new commit")
end)

log_case("n does nothing on non-commit line", function()
  local new_commit_called = false
  jj.new_commit = function(_: any, _: any, callback: function(any))
    new_commit_called = true
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 2, 0 })

  vim.api.nvim_feedkeys("n", "x", false)

  t.eq(new_commit_called, false, "jj new should not be called on non-commit line")
end)
