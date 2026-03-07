local MiniTest = require("mini.test")
local expect = MiniTest.expect
local h = require("tests.e2e.helpers")

local review = require("kenjutu.review")

local T = MiniTest.new_set()

local repo

T["e2e review"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      repo = h.create_repo()
      vim.cmd("tabnew")
    end,
    post_case = function()
      review._state = {}
      while #vim.api.nvim_list_tabpages() > 1 do
        vim.cmd("tabclose!")
      end
      if repo then
        repo.cleanup()
        repo = nil
      end
    end,
  },
})

T["e2e review"]["opens with real files and three-pane layout"] = function()
  h.write_file(repo, "alpha.txt", "alpha content\n")
  h.write_file(repo, "beta.txt", "beta content\n")
  local commit = h.jj_commit(repo, "add two files")

  local log_bufnr = vim.api.nvim_get_current_buf()
  review.open(repo.path, commit, log_bufnr, function() end)

  local file_list_bufnr = h.find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  -- Wait for file list to render (not "Loading...")
  local rendered = h.wait_until(function()
    local lines = h.buf_lines(file_list_bufnr)
    return #lines > 1 and not lines[1]:find("Loading")
  end)
  expect.equality(rendered, true)

  -- Three-pane layout: file list + left diff + right diff
  local wins = vim.api.nvim_tabpage_list_wins(0)
  expect.equality(#wins, 3)

  -- File names appear in the file list
  expect.equality(h.buf_contains(file_list_bufnr, "alpha%.txt"), true)
  expect.equality(h.buf_contains(file_list_bufnr, "beta%.txt"), true)
end

T["e2e review"]["file list shows correct review counts"] = function()
  h.write_file(repo, "one.txt", "1\n")
  h.write_file(repo, "two.txt", "2\n")
  h.write_file(repo, "three.txt", "3\n")
  local commit = h.jj_commit(repo, "add three files")

  local log_bufnr = vim.api.nvim_get_current_buf()
  review.open(repo.path, commit, log_bufnr, function() end)

  local file_list_bufnr = h.find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  local rendered = h.wait_until(function()
    return h.buf_contains(file_list_bufnr, "Files")
  end)
  expect.equality(rendered, true)

  -- 0 out of 3 reviewed initially
  expect.equality(h.buf_contains(file_list_bufnr, "Files 0/3"), true)
end

T["e2e review"]["diff panes have content after loading"] = function()
  h.write_file(repo, "diff-test.txt", "line one\nline two\nline three\n")
  local commit = h.jj_commit(repo, "add file for diff")

  local log_bufnr = vim.api.nvim_get_current_buf()
  review.open(repo.path, commit, log_bufnr, function() end)

  local file_list_bufnr = h.find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  -- Wait for file list to render
  h.wait_until(function()
    return h.buf_contains(file_list_bufnr, "Files")
  end)

  -- Get the review state to inspect diff panes
  local state = review._state[file_list_bufnr]
  expect.no_equality(state, nil)
  expect.no_equality(state.diff_state, nil)
  expect.no_equality(state.diff_state.pane, nil)

  -- Wait for diff panes to be populated
  local right_bufnr = state.diff_state.pane.right_bufnr
  local populated = h.wait_until(function()
    local lines = h.buf_lines(right_bufnr)
    return #lines > 0 and lines[1] ~= ""
  end)
  expect.equality(populated, true)

  -- Target (right) pane should contain the file content
  local lines = h.buf_lines(right_bufnr)
  local content = table.concat(lines, "\n")
  expect.no_equality(content:find("line one"), nil)
end

T["e2e review"]["toggle reviewed updates file list header"] = function()
  h.write_file(repo, "reviewable.txt", "content\n")
  local commit = h.jj_commit(repo, "add file")

  local log_bufnr = vim.api.nvim_get_current_buf()
  review.open(repo.path, commit, log_bufnr, function() end)

  local file_list_bufnr = h.find_buf_by_ft("kenjutu-review-files")
  expect.no_equality(file_list_bufnr, nil)

  -- Wait for initial render
  h.wait_until(function()
    return h.buf_contains(file_list_bufnr, "Files 0/1")
  end)

  -- Focus the file list and press space to toggle review
  local _, winnr = h.find_buf_by_ft("kenjutu-review-files")
  vim.api.nvim_set_current_win(winnr)
  -- Move cursor to the file line (header + blank + first file = line 3)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })
  vim.api.nvim_feedkeys(" ", "x", false)

  -- Wait for the header to update after mark + refresh
  local updated = h.wait_until(function()
    return h.buf_contains(file_list_bufnr, "Files 1/1")
  end)
  expect.equality(updated, true)
end

return T
