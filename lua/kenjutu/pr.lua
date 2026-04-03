local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_pr")

---@param decision string
---@return string
local function review_badge(decision)
  if decision == "APPROVED" then
    return "✓ approved"
  elseif decision == "CHANGES_REQUESTED" then
    return "✗ changes requested"
  elseif decision == "REVIEW_REQUIRED" then
    return "● review required"
  end
  return decision
end

---@class kenjutu.PrScreenState
---@field bufnr integer
---@field winnr integer
---@field dir string
---@field pr kenjutu.GhPullRequest
---@field commit_lines table<integer, kenjutu.GhCommit>
---@field body_range { first: integer, last: integer }|nil
local PrScreenState = {}
PrScreenState.__index = PrScreenState

---@param dir string
---@param pr kenjutu.GhPullRequest
---@return kenjutu.PrScreenState
function PrScreenState.new(dir, pr)
  vim.cmd("tabnew")
  local bufnr = vim.api.nvim_get_current_buf()
  local winnr = vim.api.nvim_get_current_win()

  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "hide"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-pr"

  vim.wo[winnr].cursorline = true
  vim.wo[winnr].number = false
  vim.wo[winnr].relativenumber = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false

  ---@type kenjutu.PrScreenState
  local fields = {
    bufnr = bufnr,
    winnr = winnr,
    dir = dir,
    pr = pr,
    commit_lines = {},
  }
  local self = setmetatable(fields, PrScreenState)

  self:render()
  self:setup_keymaps()

  return self
end

function PrScreenState:render()
  local pr = self.pr
  local lines = {}
  ---@type { line: integer, col_start: integer, col_end: integer, hl: string }[]
  local highlights = {}

  local title_line = string.format("#%d  %s", pr.number, pr.title)
  table.insert(lines, title_line)
  table.insert(highlights, { line = 0, col_start = 0, col_end = #title_line, hl = "Title" })

  local meta =
    string.format("%s → %s  %s  %s", pr.headRefName, pr.baseRefName, pr.author.login, review_badge(pr.reviewDecision))
  table.insert(lines, meta)
  table.insert(highlights, { line = 1, col_start = 0, col_end = #meta, hl = "Comment" })

  self.body_range = nil
  if pr.body and pr.body ~= "" then
    table.insert(lines, "")
    local body_first = #lines + 1
    for _, body_line in ipairs(vim.split(pr.body, "\n", { plain = true })) do
      table.insert(lines, body_line)
    end
    self.body_range = { first = body_first, last = #lines }
  end

  table.insert(lines, "")
  local commits_header = string.format("Commits (%d):", #pr.commits)
  table.insert(lines, commits_header)
  table.insert(highlights, { line = #lines - 1, col_start = 0, col_end = #commits_header, hl = "Comment" })
  table.insert(lines, "")

  self.commit_lines = {}
  for _, commit in ipairs(pr.commits) do
    local sha_short = commit.oid:sub(1, 8)
    local line = string.format("  %s  %s", sha_short, commit.messageHeadline)
    table.insert(lines, line)
    local line_idx = #lines
    self.commit_lines[line_idx] = commit
    table.insert(highlights, { line = line_idx - 1, col_start = 2, col_end = 2 + #sha_short, hl = "Identifier" })
  end

  vim.bo[self.bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(self.bufnr, 0, -1, false, lines)
  vim.bo[self.bufnr].modifiable = false

  vim.api.nvim_buf_clear_namespace(self.bufnr, ns, 0, -1)
  for _, hl in ipairs(highlights) do
    pcall(vim.api.nvim_buf_set_extmark, self.bufnr, ns, hl.line, hl.col_start, {
      end_col = hl.col_end,
      hl_group = hl.hl,
    })
  end

  local body_range = self.body_range
  if body_range then
    vim.wo[self.winnr].foldmethod = "manual"
    vim.wo[self.winnr].foldenable = true
    vim.api.nvim_buf_call(self.bufnr, function()
      vim.cmd(string.format("%d,%dfold", body_range.first, body_range.last))
    end)
  end

  for line_no, _ in pairs(self.commit_lines) do
    vim.api.nvim_win_set_cursor(self.winnr, { line_no, 0 })
  end
end

function PrScreenState:setup_keymaps()
  local opts = { buffer = self.bufnr, silent = true }

  vim.keymap.set("n", "q", function()
    self:close()
  end, opts)

  vim.keymap.set("n", "<CR>", function()
    local line_no = vim.api.nvim_win_get_cursor(self.winnr)[1]
    local commit = self.commit_lines[line_no]
    if commit then
      require("kenjutu.review").open(self.dir, commit.oid, self.bufnr, function() end)
    end
  end)
end

function PrScreenState:close()
  local tab_count = #vim.api.nvim_list_tabpages()
  if tab_count > 1 then
    vim.cmd("tabclose")
  elseif vim.api.nvim_buf_is_valid(self.bufnr) then
    vim.api.nvim_buf_delete(self.bufnr, { force = true })
  end
end

M.PrScreenState = PrScreenState

return M
