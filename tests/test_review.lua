---@diagnostic disable: duplicate-set-field
local t = require("tests.test")
local t_util = require("tests.utils")

local kjn = require("kenjutu.kjn")
local review = require("kenjutu.review")

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
  t_util.mock_all()
  kjn.files = function(_, change_id, cb)
    cb(nil, {
      files = mock_files,
      commitId = "abc123",
      changeId = change_id,
    })
  end
  kjn.fetch_blob = function(_, cb)
    cb(nil, "line1\nline2\nline3\n")
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

local function open_review()
  local log_bufnr = vim.api.nvim_get_current_buf()
  local commit = { change_id = "test_change", commit_id = "test_commit" }
  review.open(vim.fn.getcwd(), commit, log_bufnr, function() end)
  return log_bufnr
end

local function review_case(name, fn)
  t.run_case(name, function()
    install_mocks()
    vim.cmd("tabnew")
    local ok, err = pcall(fn)
    t_util.restore_all()
    while #vim.api.nvim_list_tabpages() > 1 do
      vim.cmd("tabclose!")
    end
    if not ok then
      error(err, 0)
    end
  end)
end

review_case("creates three-pane layout", function()
  open_review()
  local wins = vim.api.nvim_tabpage_list_wins(0)
  t.eq(#wins, 3)
end)

review_case("file list buffer has correct filetype", function()
  open_review()
  local bufnr = find_buf_by_ft("kenjutu-review-files")
  t.neq(bufnr, nil)
end)

review_case("file list keymaps are registered", function()
  open_review()
  local file_list_bufnr = find_buf_by_ft("kenjutu-review-files")
  assert(file_list_bufnr, "file list buffer not found")

  local keymaps = vim.api.nvim_buf_get_keymap(file_list_bufnr, "n")
  local expected_keys = { "<CR>", " ", "r", "t", "q" }
  for _, key in ipairs(expected_keys) do
    local found = false
    for _, km in ipairs(keymaps) do
      if km.lhs == key then
        found = true
        break
      end
    end
    t.ok(found, "expected keymap '" .. key .. "' to be registered")
  end
end)

review_case("file list renders files correctly", function()
  open_review()
  local file_list_bufnr = find_buf_by_ft("kenjutu-review-files")
  assert(file_list_bufnr, "file list buffer not found")

  local lines = vim.api.nvim_buf_get_lines(file_list_bufnr, 0, -1, false)
  t.eq(#lines, 5)
  t.neq(lines[1]:find("Files 1/2"), nil)
end)

review_case("file selection follows cursor", function()
  kjn.fetch_blob = function(opts, cb)
    cb(nil, opts.file_path)
  end
  open_review()
  local _, left_winnr = t_util.review_wins()

  local function get_left_lines()
    local bufnr = vim.api.nvim_win_get_buf(left_winnr)
    return vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  end

  vim.api.nvim_win_set_cursor(0, { 4, 0 })
  vim.cmd("doautocmd CursorMoved")
  t.eq(get_left_lines(), { mock_files[1].newPath })

  vim.api.nvim_win_set_cursor(0, { 5, 0 })
  vim.cmd("doautocmd CursorMoved")
  t.eq(get_left_lines(), { mock_files[2].newPath })
end)

review_case("close restores log buffer", function()
  local log_bufnr = open_review()

  local _, winnr = find_buf_by_ft("kenjutu-review-files")
  assert(winnr, "file list window not found")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  local cur_buf = vim.api.nvim_get_current_buf()
  t.eq(cur_buf, log_bufnr)
end)
