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

local t_util = require("tests.utils") as UtilsModule

local kjn = require("kenjutu.kjn") as any
local review = require("kenjutu.review") as any

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
} as {any}

local function install_mocks()
  t_util.mock_all()
  kjn.files = function(_: any, change_id: string, cb: function(any, any))
    cb(nil, {
      files = mock_files,
      commitId = "abc123",
      changeId = change_id,
    })
  end
  kjn.fetch_blob = function(_: any, cb: function(any, string))
    cb(nil, "line1\nline2\nline3\n")
  end
end

local function find_buf_by_ft(ft: string): integer, integer
  for _, w in ipairs(vim.api.nvim_tabpage_list_wins(0) as {integer}) do
    local b = vim.api.nvim_win_get_buf(w)
    if (vim.bo[b] as any).filetype == ft then
      return b, w
    end
  end
  return nil, nil
end

local function open_review(): integer
  local log_bufnr = vim.api.nvim_get_current_buf()
  local commit = { change_id = "test_change", commit_id = "test_commit" } as any
  review.open(vim.fn.getcwd(), commit, log_bufnr, function() end)
  return log_bufnr
end

local function review_case(name: string, fn: function())
  t.run_case(name, function()
    install_mocks()
    vim.cmd("tabnew")
    local success: boolean, err: any = pcall(fn)
    t_util.restore_all()
    while #(vim.api.nvim_list_tabpages() as {integer}) > 1 do
      vim.cmd("tabclose!")
    end
    if not success then
      error(err, 0)
    end
  end)
end

review_case("creates three-pane layout", function()
  open_review()
  local wins = vim.api.nvim_tabpage_list_wins(0) as {integer}
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

  local keymaps = vim.api.nvim_buf_get_keymap(file_list_bufnr, "n") as {any}
  local expected_keys: {string} = { "<CR>", " ", "r", "t", "q" }
  for _, key in ipairs(expected_keys) do
    local found = false
    for _, km in ipairs(keymaps) do
      if (km as any).lhs == key then
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

  local lines = vim.api.nvim_buf_get_lines(file_list_bufnr, 0, -1, false) as {string}
  t.eq(#lines, 5)
  t.neq((lines[1] as string):find("Files 1/2"), nil)
end)

review_case("file selection follows cursor", function()
  kjn.fetch_blob = function(opts: any, cb: function(any, string))
    cb(nil, opts.file_path as string)
  end
  open_review()
  local _: integer, left_winnr: integer = t_util.review_wins()

  local function get_left_lines(): {string}
    local bufnr = vim.api.nvim_win_get_buf(left_winnr)
    return vim.api.nvim_buf_get_lines(bufnr, 0, -1, false) as {string}
  end

  vim.api.nvim_win_set_cursor(0, { 4, 0 })
  vim.cmd("doautocmd CursorMoved")
  t.eq(get_left_lines(), { (mock_files[1] as any).newPath })

  vim.api.nvim_win_set_cursor(0, { 5, 0 })
  vim.cmd("doautocmd CursorMoved")
  t.eq(get_left_lines(), { (mock_files[2] as any).newPath })
end)

review_case("close restores log buffer", function()
  local log_bufnr = open_review()

  local _: integer, winnr: integer = find_buf_by_ft("kenjutu-review-files")
  assert(winnr, "file list window not found")
  vim.api.nvim_set_current_win(winnr)
  vim.api.nvim_feedkeys("q", "x", false)

  local cur_buf = vim.api.nvim_get_current_buf()
  t.eq(cur_buf, log_bufnr)
end)
