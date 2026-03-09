local t = require("tests.test")
local t_utils = require("tests.utils")
local kjn = require("kenjutu.kjn")
local review = require("kenjutu.review")

local function comments_case(name, fn)
  t.run_case(name, function()
    t_utils.mock_all()
    vim.cmd("tabnew")
    local ok, err = pcall(fn)
    t_utils.restore_all()
    while #vim.api.nvim_list_tabpages() > 1 do
      vim.cmd("tabclose!")
    end
    if not ok then
      error(err, 0)
    end
  end)
end

local function review_wins()
  local layout = vim.fn.winlayout()
  assert(layout[1] == "row", "expected row layout, got " .. layout[1])
  local children = layout[2]
  assert(#children == 3, "expected 3 children, got " .. #children)
  local file_list_winnr = children[1][2]
  local diff_left_winnr = children[2][2]
  local diff_right_winnr = children[3][2]
  assert(type(file_list_winnr) == "number", "expected file list leaf")
  assert(type(diff_left_winnr) == "number", "expected diff left leaf")
  assert(type(diff_right_winnr) == "number", "expected diff right leaf")
  return file_list_winnr, diff_left_winnr, diff_right_winnr
end

---@param bufnr integer
---@return table[]
local function get_signs(bufnr)
  local placed = vim.fn.sign_getplaced(bufnr, { group = "kenjutu_comments" })
  return placed[1] and placed[1].signs or {}
end

---@param opts { reviewStatus: string, comments: table[] }|nil
local function open_review(opts)
  opts = opts or {}
  local file = {
    newPath = "src/foo.lua",
    oldPath = "src/foo.lua",
    status = "modified",
    reviewStatus = opts.reviewStatus or "unreviewed",
    additions = 3,
    deletions = 1,
    isBinary = false,
  }

  kjn.files = function(_, _, cb)
    cb(nil, {
      files = { file },
      commitId = "abc123",
      changeId = "abc123",
    })
  end
  kjn.fetch_blob = function(_, cb)
    cb(nil, string.rep("line\n", 10))
  end
  kjn.get_comments = function(_, _, _, cb)
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
  local commit = { change_id = "test_change", commit_id = "test_commit" }
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
  })

  local _, diff_left_winnr, diff_right_winnr = review_wins()
  local marker_bufnr = vim.api.nvim_win_get_buf(diff_left_winnr)
  local target_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local target_signs = get_signs(target_bufnr)
  t.eq(#target_signs, 1)
  t.eq(target_signs[1].lnum, 2)
  t.eq(target_signs[1].name, "KenjutuComment")

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
  })

  local _, diff_left_winnr, diff_right_winnr = review_wins()
  local base_bufnr = vim.api.nvim_win_get_buf(diff_left_winnr)
  local marker_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local base_signs = get_signs(base_bufnr)
  t.eq(#base_signs, 1)
  t.eq(base_signs[1].lnum, 5)
  t.eq(base_signs[1].name, "KenjutuComment")

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
  })

  local _, _, diff_right_winnr = review_wins()
  local right_bufnr = vim.api.nvim_win_get_buf(diff_right_winnr)

  local signs = get_signs(right_bufnr)
  t.eq(#signs, 1)
  t.eq(signs[1].name, "KenjutuCommentResolved")
end)

comments_case("no signs when no comments", function()
  open_review({ comments = {} })

  local _, diff_left_winnr, diff_right_winnr = review_wins()
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
  })

  local _, _, diff_right_winnr = review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 1, 0 })

  vim.api.nvim_feedkeys("]x", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(diff_right_winnr)[1], 3)

  vim.api.nvim_feedkeys("]x", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(diff_right_winnr)[1], 7)
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
  })

  local _, _, diff_right_winnr = review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 10, 0 })

  vim.api.nvim_feedkeys("[x", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(diff_right_winnr)[1], 7)

  vim.api.nvim_feedkeys("[x", "x", false)
  t.eq(vim.api.nvim_win_get_cursor(diff_right_winnr)[1], 3)
end)

comments_case("gc creates comment with correct args", function()
  open_review({ comments = {} })

  local _, _, diff_right_winnr = review_wins()
  vim.api.nvim_set_current_win(diff_right_winnr)
  vim.api.nvim_win_set_cursor(diff_right_winnr, { 5, 0 })

  local captured_opts = nil
  kjn.add_comment = function(opts, cb)
    captured_opts = opts
    cb(nil, {})
  end

  kjn.get_comments = function(_, _, _, cb)
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
  t.eq(signs[1].lnum, 5)
end)
