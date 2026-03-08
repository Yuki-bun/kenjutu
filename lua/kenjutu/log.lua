local jj = require("kenjutu.jj")
local FileTreeState = require("kenjutu.file_tree").FileTreeState

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_log")
local squash_ns = vim.api.nvim_create_namespace("kenjutu_squash")

vim.api.nvim_set_hl(0, "KenjutuSquashSource", { bg = "#45475a", bold = true })

---@class kenjutu.SquashState
---@field source kenjutu.Commit
---@field source_line integer
---@field paths string[]|nil

---@class kenjutu.LogScreenState
---@field commits_by_line table<integer, kenjutu.Commit>
---@field commit_lines integer[]
---@field dir string
---@field file_tree kenjutu.FileTreeState|nil
---@field bufnr integer
---@field winnr integer
---@field squash_state kenjutu.SquashState|nil
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
    squash_state = nil,
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

---@class kenjutu.LogScreen.CommitAtCursorOptions
---@field cursor_line? integer defaults to the current cursor line if not provided
---@field exact? boolean if true, only return a commit if the cursor is exactly on its line

--- Find the commit at or nearest before the cursor position.
---@param opts kenjutu.LogScreen.CommitAtCursorOptions | nil
---@return kenjutu.Commit|nil
function LogScreenState:commit_at_cursor(opts)
  opts = opts or {}
  local cursor_line = opts.cursor_line or vim.api.nvim_win_get_cursor(self.winnr)[1]
  local exact = opts.exact or false
  if not cursor_line then
    cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  end
  if self.commits_by_line[cursor_line] then
    return self.commits_by_line[cursor_line]
  end
  if exact then
    return nil
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
  local current_commit = self:commit_at_cursor()
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

function LogScreenState:enter_squash_mode()
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local commit = self:commit_at_cursor({ exact = true })
  if not commit then
    return
  end
  ---@class kenjutu.SquashState
  local state = {
    source = commit,
    source_line = cursor_line,
    paths = nil,
  }
  self.squash_state = state
  self:highlight_squash_source()
  vim.api.nvim_echo({ { "Squash: select destination (s to confirm, <Esc> to cancel)", "WarningMsg" } }, false, {})
end

function LogScreenState:cancel_squash_mode()
  self.squash_state = nil
  vim.api.nvim_buf_clear_namespace(self.bufnr, squash_ns, 0, -1)
  vim.api.nvim_echo({ { "" } }, false, {})
end

function LogScreenState:highlight_squash_source()
  vim.api.nvim_buf_clear_namespace(self.bufnr, squash_ns, 0, -1)
  local squash_state = self.squash_state
  if not squash_state then
    return
  end
  local line_idx = squash_state.source_line - 1
  local line_text = vim.api.nvim_buf_get_lines(self.bufnr, line_idx, line_idx + 1, false)[1] or ""
  pcall(vim.api.nvim_buf_set_extmark, self.bufnr, squash_ns, line_idx, 0, {
    end_col = #line_text,
    hl_group = "KenjutuSquashSource",
  })
end

function LogScreenState:execute_squash()
  local dest = self:commit_at_cursor()
  if not dest then
    return
  end
  if dest.change_id == self.squash_state.source.change_id then
    vim.notify("Cannot squash a commit into itself", vim.log.levels.WARN)
    return
  end
  local source = self.squash_state.source
  if not source then
    return
  end

  local paths = self.squash_state.paths
  self:cancel_squash_mode()

  jj.squash(self.dir, {
    from = source.change_id,
    into = dest.change_id,
    paths = paths,
  }, function(err)
    if err then
      vim.notify("jj squash: " .. err, vim.log.levels.ERROR)
      return
    end
    vim.notify("Squashed into " .. dest.change_id:sub(1, 8), vim.log.levels.INFO)
    self:refresh()
  end)
end

function LogScreenState:open_squash_file_picker()
  local commit = self:commit_at_cursor()
  if not commit then
    return
  end

  jj.list_files(self.dir, commit.change_id, function(err, files)
    if err or not files then
      vim.notify("kjn files: " .. (err or "unknown error"), vim.log.levels.ERROR)
      return
    end

    if #files == 0 then
      vim.notify("No files in commit", vim.log.levels.WARN)
      return
    end

    local squash_files = require("kenjutu.squash_files")
    squash_files.open(self.winnr, files, function(selected_paths)
      if not selected_paths or #selected_paths == 0 then
        return
      end
      self:enter_squash_mode()
      self.squash_state.paths = selected_paths
    end)
  end)
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
    local commit = self:commit_at_cursor()
    if commit then
      self.file_tree:update(commit)
    end
  end

  vim.keymap.set("n", "<CR>", function()
    local commit = self:commit_at_cursor({ exact = true })
    if commit then
      if self.file_tree then
        self.file_tree:close()
        self.file_tree = nil
      end
      require("kenjutu.review").open(self.dir, commit, bufnr, on_close_review)
    end
  end, opts)

  vim.keymap.set("n", "r", function()
    self:refresh()
  end, opts)

  vim.keymap.set("n", "n", function()
    local commit = self:commit_at_cursor({ exact = true })
    if not commit then
      return
    end
    jj.new_commit(self.dir, commit.change_id, function(err)
      if err then
        vim.notify("jj new: " .. err, vim.log.levels.ERROR)
        return
      end
      self:refresh()
    end)
  end, opts)

  vim.keymap.set("n", "d", function()
    self:open_describe()
  end, opts)

  vim.keymap.set("n", "s", function()
    local cur = vim.api.nvim_win_get_cursor(0)[1]
    local commit = self.commits_by_line[cur]
    if not commit then
      return
    end

    if self.squash_state then
      if commit.change_id == self.squash_state.source.change_id then
        self:cancel_squash_mode()
        return
      end
      self:execute_squash()
    else
      self:enter_squash_mode()
    end
  end, opts)

  vim.keymap.set("n", "S", function()
    if self.squash_state then
      self:cancel_squash_mode()
    end
    self:open_squash_file_picker()
  end, opts)

  vim.keymap.set("n", "<Esc>", function()
    if self.squash_state then
      self:cancel_squash_mode()
    end
  end, opts)

  vim.keymap.set("n", "q", function()
    self:close()
  end, opts)
end

function LogScreenState:refresh()
  jj.log(self.dir, function(err, result)
    if err or result == nil then
      vim.notify("jj log: " .. err, vim.log.levels.ERROR)
      return
    end
    if not vim.api.nvim_buf_is_valid(self.bufnr) then
      return
    end
    self:render(result)
  end)
end

local describe_buf_counter = 0

function LogScreenState:open_describe()
  local commit = self:commit_at_cursor()
  if not commit then
    return
  end

  local change_id = commit.change_id

  jj.fetch_commit_metadata(self.dir, change_id, function(err, metadata)
    if err or metadata == nil then
      vim.notify("jj log metadata: " .. (err or "unknown error"), vim.log.levels.ERROR)
      return
    end

    local full_desc = metadata.summary
    if metadata.description ~= "" then
      full_desc = full_desc .. "\n" .. metadata.description
    end

    vim.api.nvim_set_current_win(self.winnr)
    vim.cmd("aboveleft split")
    local desc_winnr = vim.api.nvim_get_current_win()
    local desc_bufnr = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_win_set_buf(desc_winnr, desc_bufnr)

    vim.bo[desc_bufnr].buftype = "acwrite"
    vim.bo[desc_bufnr].filetype = "jjdescription"
    vim.bo[desc_bufnr].swapfile = false
    vim.bo[desc_bufnr].buflisted = false
    describe_buf_counter = describe_buf_counter + 1
    vim.api.nvim_buf_set_name(desc_bufnr, "describe://" .. change_id:sub(1, 8) .. "/" .. describe_buf_counter)

    local desc_lines = vim.split(full_desc, "\n", { plain = true })
    vim.api.nvim_buf_set_lines(desc_bufnr, 0, -1, false, desc_lines)
    vim.bo[desc_bufnr].modified = false

    local self_ref = self

    vim.api.nvim_create_autocmd("BufWriteCmd", {
      buffer = desc_bufnr,
      callback = function()
        local lines = vim.api.nvim_buf_get_lines(desc_bufnr, 0, -1, false)
        local message = table.concat(lines, "\n")

        jj.describe(self_ref.dir, change_id, message, function(desc_err)
          if desc_err then
            vim.notify("jj describe: " .. desc_err, vim.log.levels.ERROR)
            return
          end

          vim.bo[desc_bufnr].modified = false
          if vim.api.nvim_win_is_valid(desc_winnr) then
            vim.api.nvim_win_close(desc_winnr, true)
          end
          if vim.api.nvim_buf_is_valid(desc_bufnr) then
            vim.api.nvim_buf_delete(desc_bufnr, { force = true })
          end

          self_ref:refresh()
        end)
      end,
    })

    vim.keymap.set("n", "q", function()
      if vim.api.nvim_win_is_valid(desc_winnr) then
        vim.api.nvim_win_close(desc_winnr, true)
      end
      if vim.api.nvim_buf_is_valid(desc_bufnr) then
        vim.api.nvim_buf_delete(desc_bufnr, { force = true })
      end
    end, { buffer = desc_bufnr, silent = true })
  end)
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
      local commit = self:commit_at_cursor()
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
---@return kenjutu.LogScreenState
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

  return s
end

return M
