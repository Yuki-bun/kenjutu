local MiniTest = require("mini.test")
local expect = MiniTest.expect
local h = require("tests.e2e.helpers")

local kjn = require("kenjutu.kjn")
local FileTreeState = require("kenjutu.file_tree").FileTreeState

local T = MiniTest.new_set()

local repo
local saved_cwd

T["e2e file tree"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      repo = h.create_repo()
      saved_cwd = vim.fn.getcwd()
      vim.cmd("cd " .. vim.fn.fnameescape(repo.path))
      vim.cmd("tabnew")
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

T["e2e file tree"]["renders files from real kjn data"] = function()
  h.write_file(repo, "src/main.rs", "fn main() {}\n")
  h.write_file(repo, "src/lib.rs", "pub mod lib;\n")
  h.write_file(repo, "README.md", "# Readme\n")
  local commit = h.jj_commit(repo, "add nested files")

  local log_winnr = vim.api.nvim_get_current_win()
  local ft_state = FileTreeState.new(repo.path, log_winnr)

  ft_state:update(commit)

  -- Wait for file tree buffer to populate
  local populated = h.wait_until(function()
    local lines = h.buf_lines(ft_state.bufnr)
    return #lines > 1 and lines[1] ~= ""
  end)
  expect.equality(populated, true)

  -- Should contain file names
  expect.equality(h.buf_contains(ft_state.bufnr, "main%.rs"), true)
  expect.equality(h.buf_contains(ft_state.bufnr, "lib%.rs"), true)
  expect.equality(h.buf_contains(ft_state.bufnr, "README%.md"), true)

  -- Should contain the "Files" header
  expect.equality(h.buf_contains(ft_state.bufnr, "Files"), true)

  ft_state:close()
end

T["e2e file tree"]["shows review indicators after marking"] = function()
  h.write_file(repo, "reviewable.txt", "content\n")
  local commit = h.jj_commit(repo, "add file")

  -- Mark the file as reviewed via kjn
  h.sync(function(cb)
    kjn.run(repo.path, {
      "mark-file",
      "--change-id",
      commit.change_id,
      "--commit",
      commit.commit_id,
      "--file",
      "reviewable.txt",
    }, cb)
  end)

  local log_winnr = vim.api.nvim_get_current_win()
  local ft_state = FileTreeState.new(repo.path, log_winnr)

  ft_state:update(commit)

  -- Wait for render
  local populated = h.wait_until(function()
    return h.buf_contains(ft_state.bufnr, "reviewable%.txt")
  end)
  expect.equality(populated, true)

  -- Should show [x] for the reviewed file
  expect.equality(h.buf_contains(ft_state.bufnr, "%[x%]"), true)

  -- Header should show 1/1
  expect.equality(h.buf_contains(ft_state.bufnr, "Files 1/1"), true)

  ft_state:close()
end

return T
