local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local file_list = require("kenjutu.file_list")
local format_file_line = file_list._test.format_file_line
local review_indicator = file_list._test.review_indicator
local status_indicator = file_list._test.status_indicator

---@param overrides table|nil
---@return kenjutu.FileEntry
local function make_file(overrides)
  return vim.tbl_extend("force", {
    newPath = "file.lua",
    oldPath = "file.lua",
    status = "modified",
    reviewStatus = "unreviewed",
    additions = 0,
    deletions = 0,
  }, overrides or {})
end

-- review_indicator ------------------------------------------------------------

T["review_indicator"] = MiniTest.new_set()

T["review_indicator"]["reviewed"] = function()
  local ind, hl = review_indicator("reviewed")
  expect.equality(ind, "[x]")
  expect.equality(hl, "KenjutuReviewed")
end

T["review_indicator"]["partiallyReviewed"] = function()
  local ind, hl = review_indicator("partiallyReviewed")
  expect.equality(ind, "[~]")
  expect.equality(hl, "KenjutuPartial")
end

T["review_indicator"]["reviewedReverted"] = function()
  local ind, hl = review_indicator("reviewedReverted")
  expect.equality(ind, "[!]")
  expect.equality(hl, "KenjutuReverted")
end

T["review_indicator"]["unreviewed returns no highlight"] = function()
  local ind, hl = review_indicator("unreviewed")
  expect.equality(ind, "[ ]")
  expect.equality(hl, nil)
end

-- status_indicator ------------------------------------------------------------

T["status_indicator"] = MiniTest.new_set()

T["status_indicator"]["maps known statuses"] = function()
  local cases = {
    { "added", "A", "KenjutuStatusA" },
    { "modified", "M", "KenjutuStatusM" },
    { "deleted", "D", "KenjutuStatusD" },
    { "renamed", "R", "KenjutuStatusR" },
    { "copied", "C", "KenjutuStatusC" },
    { "typechange", "T", "KenjutuStatusT" },
  }
  for _, case in ipairs(cases) do
    local letter, hl = status_indicator(case[1])
    expect.equality(letter, case[2])
    expect.equality(hl, case[3])
  end
end

T["status_indicator"]["unknown returns ?"] = function()
  local letter, _ = status_indicator("unknown_status")
  expect.equality(letter, "?")
end

-- count_reviewed --------------------------------------------------------------

T["count_reviewed"] = MiniTest.new_set()

T["count_reviewed"]["counts only reviewed files"] = function()
  local files = {
    make_file({ reviewStatus = "reviewed" }),
    make_file({ reviewStatus = "unreviewed" }),
    make_file({ reviewStatus = "reviewed" }),
  }
  expect.equality(file_list.count_reviewed(files), 2)
end

T["count_reviewed"]["returns 0 for empty list"] = function()
  expect.equality(file_list.count_reviewed({}), 0)
end

-- format_file_line ------------------------------------------------------------

T["format_file_line"] = MiniTest.new_set()

T["format_file_line"]["produces expected text layout"] = function()
  local file = make_file({
    newPath = "src/main.lua",
    reviewStatus = "reviewed",
    status = "modified",
    additions = 10,
    deletions = 2,
  })
  local line, highlights = format_file_line(file)
  expect.no_equality(line:find("%[x%]"), nil)
  expect.no_equality(line:find("src/main.lua"), nil)
  expect.no_equality(line:find("M"), nil)
  expect.no_equality(line:find("+10"), nil)
  expect.no_equality(line:find("-2"), nil)
  expect.equality(type(highlights), "table")
end

T["format_file_line"]["omits stats when zero"] = function()
  local file = make_file({ additions = 0, deletions = 0 })
  local line, _ = format_file_line(file)
  -- Should not contain +0 or -0 stat strings
  expect.equality(line:find("%+%d"), nil)
  expect.equality(line:find("%-%d"), nil)
end

T["format_file_line"]["shows only additions when no deletions"] = function()
  local file = make_file({ additions = 5, deletions = 0 })
  local line, _ = format_file_line(file)
  expect.no_equality(line:find("+5"), nil)
  expect.equality(line:find("%-[%d]"), nil)
end

T["format_file_line"]["shows only deletions when no additions"] = function()
  local file = make_file({ additions = 0, deletions = 3 })
  local line, _ = format_file_line(file)
  expect.no_equality(line:find("-3"), nil)
end

T["format_file_line"]["highlight byte offsets are consistent"] = function()
  local file = make_file({
    newPath = "x.lua",
    reviewStatus = "reviewed",
    status = "added",
    additions = 1,
    deletions = 0,
  })
  local line, highlights = format_file_line(file)
  for _, hl in ipairs(highlights) do
    expect.equality(hl[1] >= 0, true)
    expect.equality(hl[2] <= #line, true)
    expect.equality(hl[1] < hl[2], true)
  end
end

-- render (Neovim API) ---------------------------------------------------------

T["render"] = MiniTest.new_set()

T["render"]["writes lines to buffer with header"] = function()
  local bufnr = vim.api.nvim_create_buf(false, true)
  local winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(winnr, bufnr)

  local files = {
    make_file({ newPath = "a.lua", reviewStatus = "reviewed", status = "added", additions = 1, deletions = 0 }),
    make_file({ newPath = "b.lua", reviewStatus = "unreviewed", status = "modified", additions = 3, deletions = 2 }),
  }
  file_list.render(bufnr, files, 1, winnr)

  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  -- Header line
  expect.no_equality(lines[1]:find("Files 1/2"), nil)
  -- Blank separator
  expect.equality(lines[2], "")
  -- Two file lines
  expect.equality(#lines, 4)

  -- Buffer should not be modifiable after render
  expect.equality(vim.bo[bufnr].modifiable, false)

  vim.api.nvim_buf_delete(bufnr, { force = true })
end

T["render"]["extmarks are applied at expected positions"] = function()
  local bufnr = vim.api.nvim_create_buf(false, true)
  local winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(winnr, bufnr)

  local files = {
    make_file({ newPath = "a.lua", reviewStatus = "reviewed", status = "added", additions = 1, deletions = 0 }),
  }
  file_list.render(bufnr, files, 1, winnr)

  local ns = vim.api.nvim_create_namespace("kenjutu_file_list")
  local extmarks = vim.api.nvim_buf_get_extmarks(bufnr, ns, 0, -1, { details = true })

  -- Header line (line 0) should have a KenjutuHeader extmark
  local header_marks = vim.tbl_filter(function(m)
    return m[2] == 0
  end, extmarks)
  expect.equality(#header_marks > 0, true)
  expect.equality(header_marks[1][4].hl_group, "KenjutuHeader")

  -- File line (line 2: header + blank) should have extmarks for review indicator and status
  local file_marks = vim.tbl_filter(function(m)
    return m[2] == 2
  end, extmarks)
  expect.equality(#file_marks >= 2, true)

  -- Verify at least one mark has the reviewed highlight
  local has_reviewed = false
  for _, m in ipairs(file_marks) do
    if m[4].hl_group == "KenjutuReviewed" then
      has_reviewed = true
    end
  end
  expect.equality(has_reviewed, true)

  vim.api.nvim_buf_delete(bufnr, { force = true })
end

T["render"]["positions cursor on selected file"] = function()
  local bufnr = vim.api.nvim_create_buf(false, true)
  local winnr = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_buf(winnr, bufnr)

  local files = {
    make_file({ newPath = "a.lua" }),
    make_file({ newPath = "b.lua" }),
    make_file({ newPath = "c.lua" }),
  }
  file_list.render(bufnr, files, 2, winnr)

  local cursor = vim.api.nvim_win_get_cursor(winnr)
  -- selected_index 2 + 2 (header + blank) = line 4
  expect.equality(cursor[1], 4)

  vim.api.nvim_buf_delete(bufnr, { force = true })
end

return T
