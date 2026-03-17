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

local t_utils = require("tests.utils") as UtilsModule
local kjn = require("kenjutu.kjn") as any
local review = require("kenjutu.review") as any
local comments_mod = require("kenjutu.comments") as any

local function comments_case(name: string, fn: function())
  t.run_case(name, function()
    t_utils.mock_all()
    vim.cmd("tabnew")
    local success: boolean, err: any = pcall(fn)
    t_utils.restore_all()
    while #(vim.api.nvim_list_tabpages() as {integer}) > 1 do
      vim.cmd("tabclose!")
    end
    if not success then
      error(err, 0)
    end
  end)
end

local function get_signs(bufnr: integer): {any}
  local placed = vim.fn.sign_getplaced(bufnr, { group = "kenjutu_comments" }) as {any}
  return (placed[1] as any) and (placed[1] as any).signs as {any} or {}
end

local function open_review(opts: {string:any})
  opts = opts or {}
  local file = {
    newPath = "src/foo.lua",
    oldPath = "src/foo.lua",
    status = "modified",
    reviewStatus = opts.reviewStatus or "unreviewed",
    additions = 3,
    deletions = 1,
    isBinary = false,
  } as any

  kjn.files = function(_: any, _: any, cb: function(any, any))
    cb(nil, {
      files = { file },
      commitId = "abc123",
      changeId = "abc123",
    })
  end
  kjn.fetch_blob = function(_: any, cb: function(any, string))
    cb(nil, string.rep("line\n", 10))
  end
  kjn.get_comments = function(_: any, _: any, _: any, cb: function(any, any))
    cb(nil, {
      files = {
        {
          file_path = "src/foo.lua",
          comments = opts.comments or {},
        },
      },
    })
  end

  local log_bufnr = vim.api.nvim_get_current_buf()
  local commit = { change_id = "test_change", commit_id = "test_commit" } as any
  review.open(vim.fn.getcwd(), commit, log_bufnr, function() end)
  vim.api.nvim_feedkeys("jjj", "x", false)
  vim.cmd("doautocmd CursorMoved")
end

comments_case("unreviewed file places sign on right (target) buffer", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 2,
        ported_start_line = nil,
        comment = { side = "New", resolved = false },
      },
    },
  } as {string:any})

  local _: integer, diff_left_winnr: integer, diff_right_winnr: integer = t_utils.review_wins()
  local marker_bufnr = vim.api.nvim_win_get_buf(diff_left_winnr)
  local target_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local target_signs = get_signs(target_bufnr)
  t.eq(#target_signs, 1)
  t.eq((target_signs[1] as any).lnum, 2)
  t.eq((target_signs[1] as any).name, "KenjutuComment")

  t.eq(#get_signs(marker_bufnr), 0)
end)

comments_case("reviewed file places sign on left (base) buffer", function()
  open_review({
    reviewStatus = "reviewed",
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = { side = "Old", resolved = false },
      },
    },
  } as {string:any})

  local _: integer, diff_left_winnr: integer, diff_right_winnr: integer = t_utils.review_wins()
  local base_bufnr = vim.api.nvim_win_get_buf(diff_left_winnr)
  local marker_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local base_signs = get_signs(base_bufnr)
  t.eq(#base_signs, 1)
  t.eq((base_signs[1] as any).lnum, 5)
  t.eq((base_signs[1] as any).name, "KenjutuComment")

  t.eq(#get_signs(marker_bufnr), 0)
end)

comments_case("resolved comment uses resolved sign", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 3,
        ported_start_line = nil,
        comment = { side = "New", resolved = true },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  local right_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local signs = get_signs(right_bufnr)
  t.eq(#signs, 1)
  t.eq((signs[1] as any).name, "KenjutuCommentResolved")
end)

comments_case("no signs when no comments", function()
  open_review({ comments = {} } as {string:any})

  local _: integer, diff_left_winnr: integer, diff_right_winnr: integer = t_utils.review_wins()
  local left_bufnr = vim.api.nvim_win_get_buf(diff_left_winnr)
  local right_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  t.eq(#get_signs(left_bufnr), 0)
  t.eq(#get_signs(right_bufnr), 0)
end)

comments_case("]x jumps to next comment", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 3,
        ported_start_line = nil,
        comment = { side = "New", resolved = false },
      },
      {
        is_ported = true,
        ported_line = 7,
        ported_start_line = nil,
        comment = { side = "New", resolved = false },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("]x", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 3)

  vim.api.nvim_feedkeys("]x", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 7)
end)

comments_case("[x jumps to previous comment", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 3,
        ported_start_line = nil,
        comment = { side = "New", resolved = false },
      },
      {
        is_ported = true,
        ported_line = 7,
        ported_start_line = nil,
        comment = { side = "New", resolved = false },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 10, 0 })

  vim.api.nvim_feedkeys("[x", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 7)

  vim.api.nvim_feedkeys("[x", "x", false)
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 3)
end)

comments_case("gc creates comment with correct args", function()
  open_review({ comments = {} } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 5, 0 })

  local captured_opts: any = nil
  kjn.add_comment = function(opts: any, cb: function(any, any))
    captured_opts = opts
    cb(nil, {})
  end

  kjn.get_comments = function(_: any, _: any, _: any, cb: function(any, any))
    cb(nil, {
      files = {
        {
          file_path = "src/foo.lua",
          comments = {
            {
              is_ported = true,
              ported_line = 5,
              ported_start_line = nil,
              comment = { side = "New", resolved = false },
            },
          },
        },
      },
    })
  end

  vim.api.nvim_feedkeys("gc", "x", false)

  local float_bufnr = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_lines(float_bufnr, 0, -1, false, { "test comment body" })
  vim.cmd("w")

  assert(captured_opts, "add_comment was not called")
  t.eq(captured_opts.file_path, "src/foo.lua")
  t.eq(captured_opts.side, "New")
  t.eq(captured_opts.line, 5)
  t.eq(captured_opts.body, "test comment body")

  local right_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)
  local signs = get_signs(right_bufnr)
  t.eq(#signs, 1)
  t.eq((signs[1] as any).lnum, 5)
end)

comments_case("go opens thread float and q closes it", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 5,
          start_line = nil,
          body = "this is wrong",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {
            {
              id = "r1",
              body = "fixed it",
              created_at = "2025-01-16T10:00:00Z",
              updated_at = "2025-01-16T10:00:00Z",
              edit_count = 0,
            },
          },
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 5, 0 })
  local win_count_before = #(vim.api.nvim_tabpage_list_wins(0) as {integer})

  vim.api.nvim_feedkeys("go", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected focus to move to float")
  local float_config = vim.api.nvim_win_get_config(float_winnr) as any
  assert(float_config.relative ~= "", "expected a floating window")

  local float_bufnr = vim.api.nvim_win_get_buf(float_winnr)
  local lines = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  local content = table.concat(lines, "\n")
  assert(content:find("this is wrong"), "expected root comment body")
  assert(content:find("2025%-01%-15"), "expected root comment date")
  assert(content:find("  fixed it"), "expected indented reply body")
  assert(content:find("2025%-01%-16"), "expected reply date")

  vim.api.nvim_feedkeys("q", "x", false)

  assert(not vim.api.nvim_win_is_valid(float_winnr), "expected float to be closed")
  t.eq(#(vim.api.nvim_tabpage_list_wins(0) as {integer}), win_count_before)
end)

comments_case("go on line without comment does nothing", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 5,
          start_line = nil,
          body = "some comment",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 1, 0 })
  local win_count_before = #(vim.api.nvim_tabpage_list_wins(0) as {integer})

  vim.api.nvim_feedkeys("go", "x", false)

  t.eq(vim.api.nvim_get_current_win(), diff_right_winnr)
  t.eq(#(vim.api.nvim_tabpage_list_wins(0) as {integer}), win_count_before)
end)

comments_case("gC opens comment list float", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 2,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 2,
          start_line = nil,
          body = "first comment",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
      {
        is_ported = true,
        ported_line = 7,
        ported_start_line = nil,
        comment = {
          id = "c2",
          target_sha = "abc",
          side = "New",
          line = 7,
          start_line = nil,
          body = "second comment",
          anchor = { before = {}, target = {}, after = {} },
          resolved = true,
          created_at = "2025-01-16T10:00:00Z",
          updated_at = "2025-01-16T10:00:00Z",
          edit_count = 0,
          replies = {
            {
              id = "r1",
              body = "reply",
              created_at = "2025-01-17T10:00:00Z",
              updated_at = "2025-01-17T10:00:00Z",
              edit_count = 0,
            },
          },
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  local win_count_before = #(vim.api.nvim_tabpage_list_wins(0) as {integer})

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected focus to move to float")
  local float_config = vim.api.nvim_win_get_config(float_winnr) as any
  assert(float_config.relative ~= "", "expected a floating window")

  local float_bufnr = vim.api.nvim_win_get_buf(float_winnr)
  local lines = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  local content = table.concat(lines, "\n")
  assert(content:find("first comment"), "expected first comment body")
  assert(content:find("second comment"), "expected second comment body")
  assert(content:find("%[resolved%]"), "expected resolved tag")
  assert(content:find("Reply"), "expected reply section for comment with replies")

  vim.api.nvim_feedkeys("q", "x", false)
  assert(not vim.api.nvim_win_is_valid(float_winnr), "expected float to close")
  t.eq(#(vim.api.nvim_tabpage_list_wins(0) as {integer}), win_count_before)
end)

comments_case("gC with no comments shows notification", function()
  open_review({ comments = {} } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  local win_count_before = #(vim.api.nvim_tabpage_list_wins(0) as {integer})

  local notified_msg: string = nil
  local orig_notify = vim.notify
  vim.notify = function(msg: string, _: any)
    notified_msg = msg
  end as any

  vim.api.nvim_feedkeys("gC", "x", false)

  vim.notify = orig_notify

  t.eq(vim.api.nvim_get_current_win(), diff_right_winnr)
  t.eq(#(vim.api.nvim_tabpage_list_wins(0) as {integer}), win_count_before)
  assert(
    notified_msg and notified_msg:find("No comments"),
    "expected 'No comments' notification, got: " .. tostring(notified_msg)
  )
end)

comments_case("gC enter jumps to comment line in diff", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 5,
          start_line = nil,
          body = "jump here",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected float to open")

  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true) as string, "x", false)

  assert(not vim.api.nvim_win_is_valid(float_winnr), "expected float to close")
  t.eq(vim.api.nvim_get_current_win(), diff_right_winnr)
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 5)
end)

comments_case("gC enter does not jump for Old comment in remaining mode", function()
  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "Old",
          line = 5,
          start_line = nil,
          body = "old side comment",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected float to open")

  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true) as string, "x", false)

  assert(not vim.api.nvim_win_is_valid(float_winnr), "expected float to close")
  t.eq((vim.api.nvim_win_get_cursor(diff_right_winnr) as {integer})[1], 1)
end)

comments_case("gC enter jumps to Old comment in reviewed mode", function()
  open_review({
    reviewStatus = "reviewed",
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "Old",
          line = 5,
          start_line = nil,
          body = "old side comment",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, diff_left_winnr: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected float to open")

  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<CR>", true, false, true) as string, "x", false)

  assert(not vim.api.nvim_win_is_valid(float_winnr), "expected float to close")
  t.eq(vim.api.nvim_get_current_win(), diff_left_winnr)
  t.eq((vim.api.nvim_win_get_cursor(diff_left_winnr) as {integer})[1], 5)
end)

t.run_case("build_comment_list: single comment with code snippet and reply", function()
  local pc = {
    is_ported = true,
    ported_line = 5,
    ported_start_line = nil,
    comment = {
      id = "c1",
      target_sha = "abc",
      side = "New",
      line = 5,
      start_line = nil,
      body = "fix this\nsecond line",
      anchor = { before = {}, target = { "local x = 1", "local y = 2" }, after = {} },
      resolved = false,
      created_at = "2025-01-15T10:00:00Z",
      updated_at = "2025-01-15T10:00:00Z",
      edit_count = 0,
      replies = {
        {
          id = "r1",
          body = "done",
          created_at = "2025-01-16T10:00:00Z",
          updated_at = "2025-01-16T10:00:00Z",
          edit_count = 0,
        },
      },
    },
  } as any

  local result = comments_mod.build_comment_list({ pc }, 60) as any

  t.eq(result.fold_levels[1], "0")
  assert((result.lines[1] as string):find("L5 %(New%)"), "header should contain L5 (New)")
  assert((result.lines[1] as string):find("2025%-01%-15"), "header should contain date")

  t.eq(result.lines[2], "  fix this")
  t.eq(result.fold_levels[2], ">1")

  t.eq(result.lines[3], "  second line")
  t.eq(result.fold_levels[3], "1")

  assert((result.lines[4] as string):find("local x = 1"), "first code line")
  t.eq(result.fold_levels[4], ">2")

  assert((result.lines[5] as string):find("local y = 2"), "second code line")
  t.eq(result.fold_levels[5], "2")

  assert((result.lines[6] as string):find("Reply"), "reply separator")
  t.eq(result.fold_levels[6], "1")

  t.eq(result.lines[7], "    done")
  t.eq(result.fold_levels[7], "1")

  assert((result.lines[8] as string):find("2025%-01%-16"), "reply date")
  t.eq(result.fold_levels[8], "1")

  t.eq((result.comment_fold_ranges as {any})[1].fold_start_line, 2)
  t.eq((result.comment_fold_ranges as {any})[1].resolved, false)
end)

t.run_case("build_comment_list: resolved comment without replies or code", function()
  local pc = {
    is_ported = true,
    ported_line = 3,
    ported_start_line = nil,
    comment = {
      id = "c1",
      target_sha = "abc",
      side = "Old",
      line = 3,
      start_line = nil,
      body = "looks wrong",
      anchor = { before = {}, target = {}, after = {} },
      resolved = true,
      created_at = "2025-02-01T10:00:00Z",
      updated_at = "2025-02-01T10:00:00Z",
      edit_count = 0,
      replies = {},
    },
  } as any

  local result = comments_mod.build_comment_list({ pc }, 60) as any

  t.eq(#(result.lines as {string}), 2)
  assert((result.lines[1] as string):find("%[resolved%]"), "header should have resolved tag")
  t.eq(result.fold_levels[1], "0")

  t.eq(result.lines[2], "  looks wrong")
  t.eq(result.fold_levels[2], ">1")

  t.eq((result.comment_fold_ranges as {any})[1].resolved, true)
end)

t.run_case("build_comment_list: two comments separated by blank line", function()
  local pc1 = {
    is_ported = true,
    ported_line = 2,
    ported_start_line = nil,
    comment = {
      id = "c1",
      target_sha = "abc",
      side = "New",
      line = 2,
      start_line = nil,
      body = "first",
      anchor = { before = {}, target = {}, after = {} },
      resolved = false,
      created_at = "2025-01-15T10:00:00Z",
      updated_at = "2025-01-15T10:00:00Z",
      edit_count = 0,
      replies = {},
    },
  } as any
  local pc2 = {
    is_ported = true,
    ported_line = 7,
    ported_start_line = nil,
    comment = {
      id = "c2",
      target_sha = "abc",
      side = "New",
      line = 7,
      start_line = nil,
      body = "second",
      anchor = { before = {}, target = {}, after = {} },
      resolved = false,
      created_at = "2025-01-16T10:00:00Z",
      updated_at = "2025-01-16T10:00:00Z",
      edit_count = 0,
      replies = {},
    },
  } as any

  local result = comments_mod.build_comment_list({ pc1, pc2 }, 60) as any

  t.eq(result.lines[2], "  first")
  t.eq(result.fold_levels[2], ">1")

  t.eq(result.lines[3], "")
  t.eq(result.fold_levels[3], "0")

  assert((result.lines[4] as string):find("L7"), "second header")
  t.eq(result.fold_levels[4], "0")

  t.eq(result.lines[5], "  second")
  t.eq(result.fold_levels[5], ">1")
end)

comments_case("gC x resolves comment and updates buffer in place", function()
  local resolve_opts: any = nil
  kjn.resolve_comment = function(opts: any, cb: function(any))
    resolve_opts = opts
    cb(nil)
  end

  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 5,
          start_line = nil,
          body = "needs fix",
          anchor = { before = {}, target = {}, after = {} },
          resolved = false,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  assert(float_winnr ~= diff_right_winnr, "expected float to open")

  local float_bufnr = vim.api.nvim_win_get_buf(float_winnr)
  local lines_before = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  local header_before = lines_before[1]
  assert(not header_before:find("%[resolved%]"), "should not be resolved yet")

  vim.api.nvim_feedkeys("x", "x", false)

  assert(resolve_opts, "expected resolve_comment to be called")
  t.eq(resolve_opts.comment_id, "c1")
  assert(vim.api.nvim_win_is_valid(float_winnr), "expected float to stay open")

  local lines_after = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  local header_after = lines_after[1]
  assert(header_after:find("%[resolved%]"), "expected resolved tag after x")
end)

comments_case("gC x unresolves a resolved comment", function()
  local unresolve_opts: any = nil
  kjn.unresolve_comment = function(opts: any, cb: function(any))
    unresolve_opts = opts
    cb(nil)
  end

  open_review({
    comments = {
      {
        is_ported = true,
        ported_line = 5,
        ported_start_line = nil,
        comment = {
          id = "c1",
          target_sha = "abc",
          side = "New",
          line = 5,
          start_line = nil,
          body = "was resolved",
          anchor = { before = {}, target = {}, after = {} },
          resolved = true,
          created_at = "2025-01-15T10:00:00Z",
          updated_at = "2025-01-15T10:00:00Z",
          edit_count = 0,
          replies = {},
        },
      },
    },
  } as {string:any})

  local _: integer, _: integer, diff_right_winnr: integer = t_utils.review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)

  vim.api.nvim_feedkeys("gC", "x", false)

  local float_winnr = vim.api.nvim_get_current_win()
  local float_bufnr = vim.api.nvim_win_get_buf(float_winnr)

  local lines_before = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  assert(lines_before[1]:find("%[resolved%]"), "should start resolved")

  vim.api.nvim_feedkeys("x", "x", false)

  assert(unresolve_opts, "expected unresolve_comment to be called")
  t.eq(unresolve_opts.comment_id, "c1")

  local lines_after = vim.api.nvim_buf_get_lines(float_bufnr, 0, -1, false) as {string}
  assert(not lines_after[1]:find("%[resolved%]"), "expected resolved tag removed after x")
end)
