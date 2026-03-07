---@diagnostic disable: duplicate-set-field
local t = require("tests.test")

local jj = require("kenjutu.jj")
local kjn = require("kenjutu.kjn")

local original_jj_log = jj.log
local original_jj_squash = jj.squash
local original_jj_fetch_metadata = jj.fetch_commit_metadata
local original_kjn_files = kjn.files

local mock_log_result = {
  lines = {
    "o  abc123 user commit one",
    "  first commit message",
    "o  def456 user commit two",
    "  second commit message",
    "o  ghi789 user commit three",
    "  third commit message",
  },
  highlights = {},
  commits_by_line = {
    [1] = { change_id = "aaaa1111", commit_id = "cccc1111" },
    [3] = { change_id = "bbbb2222", commit_id = "dddd2222" },
    [5] = { change_id = "cccc3333", commit_id = "eeee3333" },
  },
  commit_lines = { 1, 3, 5 },
}

local mock_files_result = {
  files = {
    {
      newPath = "src/main.rs",
      status = "modified",
      additions = 10,
      deletions = 2,
      isBinary = false,
      reviewStatus = "unreviewed",
    },
    {
      newPath = "src/lib.rs",
      status = "added",
      additions = 5,
      deletions = 0,
      isBinary = false,
      reviewStatus = "unreviewed",
    },
  },
  changeId = "aaaa1111",
  commitId = "cccc1111",
}

local function install_mocks()
  jj.log = function(_, callback)
    callback(nil, mock_log_result)
  end
  jj.fetch_commit_metadata = function(_, _, callback)
    callback(nil, { summary = "test", description = "", author = "test", timestamp = "1s ago" })
  end
  jj.squash = function(_, _, callback)
    callback(nil)
  end
  kjn.files = function(_, _, callback)
    callback(nil, mock_files_result)
  end
end

local function restore_mocks()
  jj.log = original_jj_log
  jj.squash = original_jj_squash
  jj.fetch_commit_metadata = original_jj_fetch_metadata
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

local function squash_case(name, fn)
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

-- squash mode ----------------------------------------------------------------

squash_case("s on commit enters squash mode and highlights source", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("s", "x", false)

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, {})
  t.ok(#marks > 0, "squash source line should be highlighted")
end)

squash_case("<Esc> cancels squash mode", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("s", "x", false)

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")

  vim.api.nvim_feedkeys("\27", "x", false)

  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, {})
  t.eq(#marks, 0, "squash highlight should be cleared after cancel")
end)

squash_case("s on same commit cancels squash mode", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("s", "x", false)
  vim.api.nvim_feedkeys("s", "x", false)

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, {})
  t.eq(#marks, 0, "squash mode should be cancelled when pressing s on same commit")
end)

squash_case("s on second commit executes squash with correct args", function()
  local captured_opts = nil
  jj.squash = function(_, opts, callback)
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  assert(captured_opts, "squash should have been called")
  t.eq(captured_opts.from, "aaaa1111", "source change_id should match")
  t.eq(captured_opts.into, "bbbb2222", "destination change_id should match")
  t.eq(captured_opts.paths, nil, "paths should be nil for full squash")
end)

squash_case("squash error is shown as notification", function()
  local notified_msg = nil
  local notified_level = nil
  local original_notify = vim.notify
  vim.notify = function(msg, level)
    notified_msg = msg
    notified_level = level
  end

  jj.squash = function(_, _, callback)
    callback("conflict in src/main.rs")
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("s", "x", false)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  vim.notify = original_notify

  assert(notified_msg, "error notification should be shown")
  t.ok(notified_msg:find("conflict"), "notification should contain error message")
  t.eq(notified_level, vim.log.levels.ERROR, "notification should be error level")
end)

squash_case("log refreshes after successful squash", function()
  local log_refreshed = false
  local original_log = jj.log

  jj.squash = function(_, _, callback)
    callback(nil)
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  jj.log = function(dir, callback)
    log_refreshed = true
    original_log(dir, callback)
  end

  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  vim.api.nvim_feedkeys("s", "x", false)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  t.eq(log_refreshed, true, "log should refresh after squash")
end)

-- file picker ----------------------------------------------------------------

squash_case("S opens file picker split", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local picker_bufnr = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_bufnr, "file picker should open")

  local lines = vim.api.nvim_buf_get_lines(picker_bufnr, 0, -1, false)
  t.eq(lines[1], " Select files to squash", "header should be shown")
  t.ok(#lines >= 4, "should show header + blank + file entries")
end)

squash_case("file picker: <Space> toggles file selection", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local picker_bufnr, picker_winnr = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_bufnr and picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_win_set_cursor(picker_winnr, { 3, 0 })

  vim.api.nvim_feedkeys(" ", "x", false)

  local lines = vim.api.nvim_buf_get_lines(picker_bufnr, 0, -1, false)
  t.ok(lines[3]:find("%[ %]"), "first file should be deselected after toggle")
end)

squash_case("file picker: q cancels without entering squash mode", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _, picker_winnr = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(find_buf_by_ft("kenjutu-squash-files"), nil, "file picker should close on q")

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, {})
  t.eq(#marks, 0, "squash mode should not be active after cancel")
end)

squash_case("file picker: <CR> confirms and enters squash destination mode", function()
  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _, picker_winnr = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq(find_buf_by_ft("kenjutu-squash-files"), nil, "file picker should close on confirm")

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, {})
  t.ok(#marks > 0, "squash source should be highlighted after file selection")
end)

squash_case("squash with selected files passes paths to jj squash", function()
  local captured_opts = nil
  jj.squash = function(_, opts, callback)
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _, winnr = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _, picker_winnr = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")
  vim.api.nvim_set_current_win(picker_winnr)

  -- deselect first file (src/lib.rs comes first alphabetically)
  vim.api.nvim_win_set_cursor(picker_winnr, { 3, 0 })
  vim.api.nvim_feedkeys(" ", "x", false)

  vim.api.nvim_feedkeys("\r", "x", false)

  -- now select destination
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  assert(captured_opts, "squash should have been called")
  t.eq(captured_opts.from, "aaaa1111", "source should be correct")
  t.eq(captured_opts.into, "bbbb2222", "destination should be correct")
  t.eq(#captured_opts.paths, 1, "should only include selected file")
  t.eq(captured_opts.paths[1], "src/main.rs", "should pass correct file path")
end)
