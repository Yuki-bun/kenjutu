---@diagnostic disable: duplicate-set-field
local t = require("tests.test")

local jj = require("kenjutu.jj")
local kjn = require("kenjutu.kjn")

local original_jj_log = jj.log
local original_jj_fetch_metadata = jj.fetch_commit_metadata
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
  kjn.files = function(_, _, callback)
    callback(nil, { files = {}, changeId = "abc123", commitId = "abc123" })
  end
end

local function restore_mocks()
  jj.log = original_jj_log
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
