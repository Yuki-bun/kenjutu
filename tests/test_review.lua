---@diagnostic disable: duplicate-set-field
local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local kjn = require("kenjutu.kjn")
local review = require("kenjutu.review")

local original_kjn_run = kjn.run
local original_kjn_run_raw = kjn.run_raw

local mock_files = {
  {
    newPath = "src/main.lua",
    oldPath = "src/main.lua",
    status = "modified",
    reviewStatus = "unreviewed",
    additions = 5,
    deletions = 2,
    isBinary = false,
  },
  {
    newPath = "src/utils.lua",
    oldPath = "src/utils.lua",
    status = "added",
    reviewStatus = "reviewed",
    additions = 10,
    deletions = 0,
    isBinary = false,
  },
}

local function install_mocks()
  kjn.run = function(_, args, callback)
    if args[1] == "files" then
      callback(nil, { files = mock_files, commitId = "abc123" })
    else
      callback(nil, {})
    end
  end
  kjn.run_raw = function(_, _, callback)
    callback(nil, "line1\nline2\nline3\n")
  end
end

local function restore_mocks()
  kjn.run = original_kjn_run
  kjn.run_raw = original_kjn_run_raw
end

--- Find a buffer by filetype in the current tab.
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

local function open_review()
  local log_bufnr = vim.api.nvim_get_current_buf()
  local commit = { change_id = "test_change", commit_id = "test_commit" }
  local closed = false
  review.open(vim.fn.getcwd(), commit, log_bufnr, function()
    closed = true
  end)
  return log_bufnr, closed
end

T["review"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      install_mocks()
      vim.cmd("tabnew")
    end,
    post_case = function()
      restore_mocks()
      review._state = {}
      while #vim.api.nvim_list_tabpages() > 1 do
        vim.cmd("tabclose!")
      end
    end,
  },
})

T["review"]["creates three-pane layout"] = function()
  open_review()
  local wins = vim.api.nvim_tabpage_list_wins(0)
  expect.equality(#wins, 3)
end

T["review"]["file list buffer has correct filetype"] = function()
  open_review()
  local bufnr = find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(bufnr, nil)
end

T["review"]["file list keymaps are registered"] = function()
  open_review()
  local file_list_bufnr = find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  local keymaps = vim.api.nvim_buf_get_keymap(file_list_bufnr, "n")
  local expected_keys = { "j", "k", "<CR>", " ", "r", "t", "q" }
  for _, key in ipairs(expected_keys) do
    local found = false
    for _, km in ipairs(keymaps) do
      if km.lhs == key then
        found = true
        break
      end
    end
    expect.equality(found, true)
  end
end

T["review"]["file list renders files correctly"] = function()
  open_review()
  local file_list_bufnr = find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  local lines = vim.api.nvim_buf_get_lines(file_list_bufnr, 0, -1, false)
  -- Header + blank + 2 files = 4 lines
  expect.equality(#lines, 4)
  expect.no_equality(lines[1]:find("Files 1/2"), nil)
end

T["review"]["close restores log buffer"] = function()
  local log_bufnr = open_review()

  local _, winnr = find_buf_by_ft("kenjutu-review-files")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  local cur_buf = vim.api.nvim_get_current_buf()
  expect.equality(cur_buf, log_bufnr)
end

return T
