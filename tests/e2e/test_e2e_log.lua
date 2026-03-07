local MiniTest = require("mini.test")
local expect = MiniTest.expect
local h = require("tests.e2e.helpers")

local T = MiniTest.new_set()

local repo
local saved_cwd

T["e2e log"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      repo = h.create_repo()
      saved_cwd = vim.fn.getcwd()
      vim.cmd("cd " .. vim.fn.fnameescape(repo.path))
    end,
    post_case = function()
      while #vim.api.nvim_list_tabpages() > 1 do
        vim.cmd("tabclose!")
      end
      if saved_cwd then
        vim.cmd("cd " .. vim.fn.fnameescape(saved_cwd))
      end
      if repo then
        repo.cleanup()
        repo = nil
      end
    end,
  },
})

T["e2e log"]["renders commits from real jj log"] = function()
  h.write_file(repo, "first.txt", "first\n")
  h.jj_commit(repo, "first commit")

  h.write_file(repo, "second.txt", "second\n")
  h.jj_commit(repo, "second commit")

  require("kenjutu.log").open()

  local log_bufnr = h.find_buf_by_ft("kenjutu-log")
  expect.no_equality(log_bufnr, nil)

  -- Wait for log to render (not "Loading...")
  local rendered = h.wait_until(function()
    local lines = h.buf_lines(log_bufnr)
    return #lines > 1 and not lines[1]:find("Loading")
  end)
  expect.equality(rendered, true)

  -- Buffer should have commit message text
  local lines = h.buf_lines(log_bufnr)
  expect.equality(#lines > 2, true)

  local all_text = table.concat(lines, "\n")
  expect.no_equality(all_text:find("first commit"), nil)
  expect.no_equality(all_text:find("second commit"), nil)
end

T["e2e log"]["file tree sidebar populates on cursor move"] = function()
  h.write_file(repo, "sidebar.txt", "sidebar content\n")
  h.jj_commit(repo, "add sidebar file")

  require("kenjutu.log").open()

  local log_bufnr = h.find_buf_by_ft("kenjutu-log")
  expect.no_equality(log_bufnr, nil)

  -- Wait for log to render
  h.wait_until(function()
    local lines = h.buf_lines(log_bufnr)
    return #lines > 1 and not lines[1]:find("Loading")
  end)

  -- File tree sidebar should exist
  local ft_bufnr = h.find_buf_by_ft("kenjutu-log-files")
  expect.no_equality(ft_bufnr, nil)

  -- Wait for file tree to populate (it updates via CursorMoved autocmd)
  local populated = h.wait_until(function()
    return h.buf_contains(ft_bufnr, "sidebar%.txt")
  end)
  expect.equality(populated, true)
end

return T
