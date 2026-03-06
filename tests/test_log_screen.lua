---@diagnostic disable: duplicate-set-field
local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local jj = require("kenjutu.jj")
local kjn = require("kenjutu.kjn")

local original_jj_log = jj.log
local original_jj_fetch_metadata = jj.fetch_commit_metadata
local original_kjn_run = kjn.run

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
  kjn.run = function(_, _, callback)
    callback(nil, { files = {}, commitId = "abc123" })
  end
end

local function restore_mocks()
  jj.log = original_jj_log
  jj.fetch_commit_metadata = original_jj_fetch_metadata
  kjn.run = original_kjn_run
end

local function cleanup_tabs()
  while #vim.api.nvim_list_tabpages() > 1 do
    vim.cmd("tabclose!")
  end
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

T["log screen"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      install_mocks()
    end,
    post_case = function()
      restore_mocks()
      cleanup_tabs()
    end,
  },
})

T["log screen"]["open creates a new tab"] = function()
  local tabs_before = #vim.api.nvim_list_tabpages()
  require("kenjutu.log").open()
  expect.equality(#vim.api.nvim_list_tabpages(), tabs_before + 1)
end

T["log screen"]["log buffer has correct filetype"] = function()
  require("kenjutu.log").open()
  local bufnr = find_buf_by_ft("kenjutu-log")
  expect.no_equality(bufnr, nil)
end

T["log screen"]["file tree sidebar opens"] = function()
  require("kenjutu.log").open()
  local bufnr = find_buf_by_ft("kenjutu-log-files")
  expect.no_equality(bufnr, nil)
end

T["log screen"]["tab has two windows"] = function()
  require("kenjutu.log").open()
  local wins = vim.api.nvim_tabpage_list_wins(0)
  expect.equality(#wins, 2)
end

T["log screen"]["keymaps are registered on log buffer"] = function()
  require("kenjutu.log").open()
  local log_bufnr = find_buf_by_ft("kenjutu-log")
  expect.no_equality(log_bufnr, nil)

  local keymaps = vim.api.nvim_buf_get_keymap(log_bufnr, "n")
  local expected_keys = { "j", "k", "<CR>", "r", "q" }
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

T["log screen"]["q closes the tab"] = function()
  require("kenjutu.log").open()
  local tabs_before = #vim.api.nvim_list_tabpages()

  local _, winnr = find_buf_by_ft("kenjutu-log")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  expect.equality(#vim.api.nvim_list_tabpages(), tabs_before - 1)
end

return T
