local jj = require("kenjutu.jj")
local FileTreeState = require("kenjutu.file_tree").FileTreeState

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_log")

---@class kenjutu.LogScreenState
---@field commits_by_line table<integer, kenjutu.Commit>
---@field commit_lines integer[]
---@field dir string
---@field file_tree kenjutu.FileTreeState|nil
---@field bufnr integer
---@field winnr integer
local LogScreenState = {}
LogScreenState.__index = LogScreenState

function LogScreenState.new()
  local bufnr = vim.api.nvim_get_current_buf()
  local winnr = vim.api.nvim_get_current_win()

  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "hide"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-log"

  vim.wo[winnr].cursorline = true
  vim.wo[winnr].number = false
  vim.wo[winnr].relativenumber = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false

  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "Loading..." })
  vim.bo[bufnr].modifiable = false

  ---@type kenjutu.LogScreenState
  local fields = {
    commits_by_line = {},
    commit_lines = {},
    dir = vim.fn.getcwd(),
    file_tree = nil,
    bufnr = bufnr,
    winnr = winnr,
  }
  return setmetatable(fields, LogScreenState)
end

function LogScreenState:goto_next_commit()
  local current = vim.api.nvim_win_get_cursor(0)[1]
  for _, line_no in ipairs(self.commit_lines) do
    if line_no > current then
      vim.api.nvim_win_set_cursor(0, { line_no, 0 })
      break
    end
  end
end

function LogScreenState:goto_prev_commit()
  local current = vim.api.nvim_win_get_cursor(0)[1]
  for i = #self.commit_lines, 1, -1 do
    if self.commit_lines[i] < current then
      vim.api.nvim_win_set_cursor(0, { self.commit_lines[i], 0 })
      break
    end
  end
end

--- Find the commit at or nearest before the cursor position.
---@param cursor_line integer
---@return kenjutu.Commit|nil
function LogScreenState:commit_at_cursor(cursor_line)
  if self.commits_by_line[cursor_line] then
    return self.commits_by_line[cursor_line]
  end
  local nearest = nil
  for _, line_no in ipairs(self.commit_lines) do
    if line_no <= cursor_line then
      nearest = line_no
    else
      break
    end
  end
  if nearest then
    return self.commits_by_line[nearest]
  end
  return nil
end

--- Render parsed jj log output into the buffer with syntax highlighting.
---@param result kenjutu.LogResult
function LogScreenState:render(result)
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local current_commit = self:commit_at_cursor(cursor_line)
  local prev_change_id = current_commit and current_commit.change_id or nil

  local bufnr = self.bufnr
  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, result.lines)
  vim.bo[bufnr].modifiable = false

  self.commits_by_line = result.commits_by_line
  self.commit_lines = result.commit_lines

  -- Apply extmark highlights from parsed ANSI data
  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  if result.highlights then
    for line_idx, spans in pairs(result.highlights) do
      for _, span in ipairs(spans) do
        pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, line_idx - 1, span.col_start, {
          end_col = span.col_end,
          hl_group = span.hl_group,
        })
      end
    end
  end

  local restored = false
  if prev_change_id then
    for _, line_no in ipairs(result.commit_lines) do
      if result.commits_by_line[line_no].change_id == prev_change_id then
        vim.api.nvim_win_set_cursor(0, { line_no, 0 })
        restored = true
        break
      end
    end
  end
  if not restored then
    local first_commit_line = result.commit_lines[1]
    if first_commit_line then
      vim.api.nvim_win_set_cursor(0, { first_commit_line, 0 })
    end
  end
end

function LogScreenState:setup_keymaps()
  local bufnr = self.bufnr
  local opts = { buffer = bufnr, silent = true }

  vim.keymap.set("n", "j", function()
    self:goto_next_commit()
  end, opts)

  vim.keymap.set("n", "k", function()
    self:goto_prev_commit()
  end, opts)

  local function on_close_review()
    self.file_tree = FileTreeState.new(self.dir, self.winnr)
    local commit = self:commit_at_cursor(vim.api.nvim_win_get_cursor(self.winnr)[1])
    if commit then
      self.file_tree:update(commit)
    end
  end

  vim.keymap.set("n", "<CR>", function()
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local commit = self.commits_by_line[cur]
    if commit then
      if self.file_tree then
        self.file_tree:close()
        self.file_tree = nil
      end
      require("kenjutu.review").open(self.dir, commit, bufnr, on_close_review)
    end
  end, opts)

  vim.keymap.set("n", "r", function()
    jj.log(self.dir, function(err, result)
      if err or result == nil then
        vim.notify("jj log: " .. err, vim.log.levels.ERROR)
        return
      end
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end
      self:render(result)
    end)
  end, opts)

  vim.keymap.set("n", "q", function()
    self:close()
  end, opts)
end

function LogScreenState:close()
  if self.file_tree then
    self.file_tree:close()
    self.file_tree = nil
  end
  local tab_count = #vim.api.nvim_list_tabpages()
  if tab_count > 1 then
    vim.cmd("tabclose")
  elseif vim.api.nvim_buf_is_valid(self.bufnr) then
    vim.api.nvim_buf_delete(self.bufnr, { force = true })
  end
end

function LogScreenState:setup_cursor_follow()
  local bufnr = self.bufnr
  vim.api.nvim_create_autocmd("CursorMoved", {
    buffer = bufnr,
    callback = function()
      local cur = vim.api.nvim_win_get_cursor(0)[1]
      local commit = self:commit_at_cursor(cur)
      if not commit then
        return
      end

      if not self.file_tree then
        self.file_tree = FileTreeState.new(self.dir, self.winnr)
      end

      self.file_tree:update(commit)
    end,
  })
end

--- Open the commit log screen in a new tab.
function M.open()
  vim.cmd("tabnew")
  local s = LogScreenState.new()

  s.file_tree = FileTreeState.new(s.dir, s.winnr)
  vim.api.nvim_set_current_win(s.winnr)

  s:setup_keymaps()
  s:setup_cursor_follow()

  jj.log(s.dir, function(err, result)
    if err or result == nil then
      vim.notify("jj log: " .. err, vim.log.levels.ERROR)
      return
    end
    if not vim.api.nvim_buf_is_valid(s.bufnr) then
      return
    end
    s:render(result)
  end)

  vim.api.nvim_create_autocmd("BufWipeout", {
    buffer = s.bufnr,
    once = true,
    callback = function()
      s:close()
    end,
  })
end

return M
