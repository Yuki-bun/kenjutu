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
} as any

local mock_files = {
  {
    path = "src/main.rs",
    status = "modified",
  },
  {
    path = "src/lib.rs",
    status = "added",
  },
} as {any}

local empty_metadata = { summary = "", description = "", author = "test", timestamp = "1s ago" } as any

local function install_mocks()
  t_util.mock_all()
  jj.log = function(_: any, callback: function(any, any))
    callback(nil, mock_log_result)
  end
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "test", description = "", author = "test", timestamp = "1s ago" })
  end
  jj.list_files = function(_: any, _: any, callback: function(any, any))
    callback(nil, mock_files)
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

local function squash_case(name: string, fn: function())
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

local function do_squash(winnr: integer, source_line: integer, dest_line: integer)
  vim.api.nvim_win_set_cursor(winnr, { source_line, 0 })
  vim.api.nvim_feedkeys("s", "x", false)
  vim.api.nvim_win_set_cursor(winnr, { dest_line, 0 })
  vim.api.nvim_feedkeys("s", "x", false)
end

squash_case("s on commit enters squash mode and highlights source", function()
  local state = require("kenjutu.log").open() as any
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("s", "x", false)

  assert(state.squash_state, "squash_state should be set")
  t.eq(state.squash_state.source.change_id, "aaaa1111", "source should be the commit under cursor")

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")
  local marks = vim.api.nvim_buf_get_extmarks(bufnr, squash_ns, 0, -1, { details = true }) as {any}
  t.eq(#marks, 1, "should have one highlight extmark on source line")
  t.eq((marks[1] as {any})[2], 0, "extmark should be on line 1 (0-indexed)")
  t.eq(((marks[1] as {any})[4] as any).hl_group, "KenjutuSquashSource", "extmark should use KenjutuSquashSource highlight")
end)

squash_case("<Esc> cancels squash mode", function()
  local state = require("kenjutu.log").open() as any
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("s", "x", false)
  assert(state.squash_state, "squash_state should be set after first s")

  vim.api.nvim_feedkeys("\27", "x", false)

  t.eq(state.squash_state, nil, "squash_state should be nil after cancel")
end)

squash_case("s on same commit cancels squash mode", function()
  local state = require("kenjutu.log").open() as any
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  local squash_called = false
  jj.squash = function(_: any, _: any, callback: function(any))
    callback(nil)
    squash_called = true
  end

  vim.api.nvim_feedkeys("s", "x", false)
  assert(squash_called == false, "jj.squash should not be called when pressing s on same commit")
  assert(state.squash_state, "squash_state should be set after first s")

  vim.api.nvim_feedkeys("s", "x", false)

  t.eq(state.squash_state, nil, "squash_state should be nil when pressing s on same commit")
end)

squash_case("s on second commit executes squash with correct args", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, empty_metadata)
  end

  local captured_opts: any = nil
  jj.squash = function(_: any, opts: any, callback: function(any))
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  assert(captured_opts, "squash should have been called")
  t.eq(captured_opts.from, "aaaa1111", "source change_id should match")
  t.eq(captured_opts.into, "bbbb2222", "destination change_id should match")
  t.eq(captured_opts.paths, nil, "paths should be nil for full squash")
  t.eq(captured_opts.message, nil, "message should be nil for empty descriptions")
  t.eq(find_buf_by_ft("jjdescription"), nil, "editor should not open")
end)

squash_case("squash error is shown as notification", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, empty_metadata)
  end

  local notified_msg: string = nil
  local notified_level: integer = nil
  local original_notify = vim.notify
  vim.notify = function(msg: string, level: integer)
    notified_msg = msg
    notified_level = level
  end as any

  jj.squash = function(_: any, _: any, callback: function(any))
    callback("conflict in src/main.rs")
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  vim.notify = original_notify

  assert(notified_msg, "error notification should be shown")
  t.ok(notified_msg:find("conflict"), "notification should contain error message")
  t.eq(notified_level, vim.log.levels.ERROR, "notification should be error level")
end)

squash_case("log refreshes after successful squash", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, empty_metadata)
  end

  jj.squash = function(_: any, _: any, callback: function(any))
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)

  local bufnr = vim.api.nvim_win_get_buf(winnr)
  local lines_before = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false) as {string}
  t.eq(#lines_before, 6, "should start with 6 lines")

  local post_squash_result = {
    lines = {
      "o  def456 user commit two (squashed)",
      "  second commit message",
      "o  ghi789 user commit three",
      "  third commit message",
    },
    highlights = {},
    commits_by_line = {
      [1] = { change_id = "bbbb2222", commit_id = "dddd2222" },
      [3] = { change_id = "cccc3333", commit_id = "eeee3333" },
    },
    commit_lines = { 1, 3 },
  } as any

  jj.log = function(_: any, callback: function(any, any))
    callback(nil, post_squash_result)
  end

  do_squash(winnr, 1, 3)

  local lines_after = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false) as {string}
  t.eq(#lines_after, 4, "should have 4 lines after squash removed a commit")
  t.eq(lines_after[1], "o  def456 user commit two (squashed)", "first line should reflect post-squash log")
  t.eq(lines_after[3], "o  ghi789 user commit three", "second commit should still be present")
end)

squash_case("S opens file picker split above", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local picker_bufnr: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_bufnr and picker_winnr, "file picker should open")

  local lines = vim.api.nvim_buf_get_lines(picker_bufnr, 0, -1, false) as {string}
  t.eq(lines[1], " Select files to squash", "header should be shown")
  t.ok(#lines >= 4, "should show header + blank + file entries")

  local picker_row = (vim.api.nvim_win_get_position(picker_winnr) as {integer})[1]
  local picker_height = vim.api.nvim_win_get_height(picker_winnr)
  local log_row = (vim.api.nvim_win_get_position(winnr) as {integer})[1]
  t.eq(picker_row + picker_height + 1, log_row, "file picker should be directly above the log window")
end)

squash_case("file picker: <Space> toggles file selection", function()
  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local picker_bufnr: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_bufnr and picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_win_set_cursor(picker_winnr, { 3, 0 })

  vim.api.nvim_feedkeys(" ", "x", false)

  local lines = vim.api.nvim_buf_get_lines(picker_bufnr, 0, -1, false) as {string}
  t.ok(lines[3]:find("%[ %]"), "first file should be deselected after toggle")
end)

squash_case("file picker: q cancels without entering squash mode", function()
  local state = require("kenjutu.log").open() as any
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(find_buf_by_ft("kenjutu-squash-files"), nil, "file picker should close on q")
  t.eq(state.squash_state, nil, "squash_state should be nil after cancel")
end)

squash_case("file picker: <CR> confirms and enters squash destination mode", function()
  local state = require("kenjutu.log").open() as any
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")

  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_feedkeys("\r", "x", false)

  t.eq(find_buf_by_ft("kenjutu-squash-files"), nil, "file picker should close on confirm")
  assert(state.squash_state, "squash_state should be set after file selection")
  t.eq(state.squash_state.source.change_id, "aaaa1111", "source should be the commit under cursor")
end)

squash_case("full squash after cancelled selective squash should not leak stale paths", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, empty_metadata)
  end

  local captured_opts: any = nil
  jj.squash = function(_: any, opts: any, callback: function(any))
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")
  vim.api.nvim_set_current_win(picker_winnr)
  vim.api.nvim_feedkeys("\r", "x", false)

  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("\27", "x", false)

  do_squash(winnr, 1, 3)

  assert(captured_opts, "squash should have been called")
  t.eq(captured_opts.from, "aaaa1111", "source should be correct")
  t.eq(captured_opts.into, "bbbb2222", "destination should be correct")
  t.eq(captured_opts.paths, nil, "paths should be nil for full squash after cancelled selective squash")
end)

squash_case("squash with selected files passes paths to jj squash", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, empty_metadata)
  end

  local captured_opts: any = nil
  jj.squash = function(_: any, opts: any, callback: function(any))
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 1, 0 })

  vim.api.nvim_feedkeys("S", "x", false)

  local _: integer, picker_winnr: integer = find_buf_by_ft("kenjutu-squash-files")
  assert(picker_winnr, "file picker should open")
  vim.api.nvim_set_current_win(picker_winnr)

  vim.api.nvim_win_set_cursor(picker_winnr, { 3, 0 })
  vim.api.nvim_feedkeys(" ", "x", false)

  vim.api.nvim_feedkeys("\r", "x", false)

  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys("s", "x", false)

  assert(captured_opts, "squash should have been called")
  t.eq(captured_opts.from, "aaaa1111", "source should be correct")
  t.eq(captured_opts.into, "bbbb2222", "destination should be correct")
  t.eq(#(captured_opts.paths as {string}), 1, "should only include selected file")
  t.eq((captured_opts.paths as {string})[1], "src/main.rs", "should pass correct file path")
end)

squash_case("squash with only one description passes message directly", function()
  jj.fetch_commit_metadata = function(_: any, change_id: string, callback: function(any, any))
    if change_id == "aaaa1111" then
      callback(nil, { summary = "source summary", description = "source body", author = "test", timestamp = "1s ago" })
    else
      callback(nil, empty_metadata)
    end
  end

  local captured_opts: any = nil
  jj.squash = function(_: any, opts: any, callback: function(any))
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  assert(captured_opts, "squash should have been called immediately")
  t.eq(captured_opts.message, "source summary\nsource body", "message should include full source description")
  t.eq(find_buf_by_ft("jjdescription"), nil, "editor should not open")
end)

squash_case("squash with both descriptions opens editor with JJ: separators", function()
  jj.fetch_commit_metadata = function(_: any, change_id: string, callback: function(any, any))
    if change_id == "aaaa1111" then
      callback(nil, { summary = "source summary", description = "source body", author = "test", timestamp = "1s ago" })
    else
      callback(nil, { summary = "dest summary", description = "dest body", author = "test", timestamp = "1s ago" })
    end
  end

  jj.squash = function(_: any, _: any, callback: function(any))
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  local desc_bufnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "editor should open when both commits have descriptions")

  local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false) as {string}
  t.eq(lines[1], "JJ: Enter a description for the combined commit.", "first line should be JJ header")
  t.eq(lines[2], "JJ: Description from the destination commit:", "second line should be dest header")
  t.eq(lines[3], "dest summary", "third line should be dest summary")
  t.eq(lines[4], "dest body", "fourth line should be dest body")
  t.eq(lines[5], "", "fifth line should be blank separator")
  t.eq(lines[6], "JJ: Description from source commit:", "sixth line should be source header")
  t.eq(lines[7], "source summary", "seventh line should be source summary")
  t.eq(lines[8], "source body", "eighth line should be source body")
end)

squash_case("squash message editor :w strips JJ: lines and executes squash", function()
  jj.fetch_commit_metadata = function(_: any, change_id: string, callback: function(any, any))
    if change_id == "aaaa1111" then
      callback(nil, { summary = "source msg", description = "", author = "test", timestamp = "1s ago" })
    else
      callback(nil, { summary = "dest msg", description = "", author = "test", timestamp = "1s ago" })
    end
  end

  local captured_opts: any = nil
  jj.squash = function(_: any, opts: any, callback: function(any))
    captured_opts = opts
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  local desc_bufnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_bufnr, "editor should open")

  vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, {
    "JJ: Enter a description for the combined commit.",
    "JJ: Description from the destination commit:",
    "combined message",
    "JJ: Description from source commit:",
  })
  vim.api.nvim_set_current_buf(desc_bufnr)
  vim.cmd("write")

  assert(captured_opts, "squash should have been called after :w")
  t.eq(captured_opts.message, "combined message", "JJ: lines should be stripped from message")
  t.eq(captured_opts.from, "aaaa1111", "source should match")
  t.eq(captured_opts.into, "bbbb2222", "destination should match")
  t.eq(find_buf_by_ft("jjdescription"), nil, "editor should close after save")
end)

squash_case("squash message editor q cancels without squashing", function()
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "has msg", description = "", author = "test", timestamp = "1s ago" })
  end

  local squash_called = false
  jj.squash = function(_: any, _: any, callback: function(any))
    squash_called = true
    callback(nil)
  end

  require("kenjutu.log").open()
  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-log")
  assert(winnr, "could not find log window")
  vim.api.nvim_set_current_win(winnr)
  do_squash(winnr, 1, 3)

  local _: integer, desc_winnr: integer = find_buf_by_ft("jjdescription")
  assert(desc_winnr, "editor should open")

  vim.api.nvim_set_current_win(desc_winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  t.eq(squash_called, false, "squash should not be called when editor is cancelled")
  t.eq(find_buf_by_ft("jjdescription"), nil, "editor should close")
end)
