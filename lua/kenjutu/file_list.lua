local utils = require("kenjutu.utils")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_file_list")

-- Highlight groups ----------------------------------------------------------

local hl_defs = {
  KenjutuReviewed = { fg = "#a6e3a1" },
  KenjutuPartial = { fg = "#f9e2af" },
  KenjutuReverted = { fg = "#6c7086" },
  KenjutuStatusA = { fg = "#a6e3a1" },
  KenjutuStatusM = { fg = "#f9e2af" },
  KenjutuStatusD = { fg = "#f38ba8" },
  KenjutuStatusR = { fg = "#89b4fa" },
  KenjutuStatusC = { fg = "#94e2d5" },
  KenjutuStatusT = { fg = "#cba6f7" },
  KenjutuStats = { fg = "#6c7086" },
  KenjutuHeader = { fg = "#cdd6f4", bold = true },
}

for name, def in pairs(hl_defs) do
  vim.api.nvim_set_hl(0, name, def)
end

--- Map review status to bracket indicator and highlight group.
---@param status string
---@return string indicator
---@return string|nil hl_group
local function review_indicator(status)
  if status == "reviewed" then
    return "[x]", "KenjutuReviewed"
  elseif status == "partiallyReviewed" then
    return "[~]", "KenjutuPartial"
  elseif status == "reviewedReverted" then
    return "[!]", "KenjutuReverted"
  else
    return "[ ]", nil
  end
end

--- Map file change status to a letter and highlight group.
---@param status string
---@return string letter
---@return string hl_group
local function status_indicator(status)
  local map = {
    added = { "A", "KenjutuStatusA" },
    modified = { "M", "KenjutuStatusM" },
    deleted = { "D", "KenjutuStatusD" },
    renamed = { "R", "KenjutuStatusR" },
    copied = { "C", "KenjutuStatusC" },
    typechange = { "T", "KenjutuStatusT" },
  }
  local entry = map[status]
  if entry then
    return entry[1], entry[2]
  end
  return "?", "KenjutuStats"
end

--- Count reviewed files.
---@param files kenjutu.FileEntry[]
---@return integer
function M.count_reviewed(files)
  local n = 0
  for _, f in ipairs(files) do
    if f.reviewStatus == "reviewed" then
      n = n + 1
    end
  end
  return n
end

-- Rendering -----------------------------------------------------------------

--- Build a plain text line for one file entry.
--- Returns the line string and a list of {col_start, col_end, hl_group} tuples.
---@param file kenjutu.FileEntry
---@return string line
---@return {[1]: integer, [2]: integer, [3]: string}[] highlights
local function format_file_line(file)
  local indicator, indicator_hl = review_indicator(file.reviewStatus)
  local path = utils.file_path(file)
  local status_char, status_hl = status_indicator(file.status)

  -- Build the line:  "[x]  path M +N -M"
  local parts = {}
  local highlights = {}
  local col = 0

  -- Review indicator
  table.insert(parts, indicator)
  if indicator_hl then
    table.insert(highlights, { col, col + #indicator, indicator_hl })
  end
  col = col + #indicator

  -- Two spaces separator
  table.insert(parts, "  ")
  col = col + 2

  -- File path (default color)
  table.insert(parts, path)
  col = col + #path

  -- Space + status letter
  local status_str = " " .. status_char
  table.insert(parts, status_str)
  -- Highlight just the letter, not the space
  table.insert(highlights, { col + 1, col + 1 + #status_char, status_hl })
  col = col + #status_str

  -- Stats
  if file.additions > 0 or file.deletions > 0 then
    local stats = ""
    if file.additions > 0 then
      stats = stats .. " +" .. file.additions
    end
    if file.deletions > 0 then
      stats = stats .. " -" .. file.deletions
    end
    table.insert(parts, stats)
    table.insert(highlights, { col, col + #stats, "KenjutuStats" })
    col = col + #stats
  end

  return table.concat(parts), highlights
end

--- Render the file list into the buffer.
---@param bufnr integer
---@param files kenjutu.FileEntry[]
---@param selected_index integer  1-indexed
---@param winnr integer  file list window (for cursor positioning)
function M.render(bufnr, files, selected_index, winnr)
  local lines = {}
  local all_highlights = {} -- [line_index] = highlights

  -- Header line
  local reviewed = M.count_reviewed(files)
  local header = string.format(" Files %d/%d", reviewed, #files)
  table.insert(lines, header)
  table.insert(all_highlights, { { 0, #header, "KenjutuHeader" } })

  -- Blank separator
  table.insert(lines, "")
  table.insert(all_highlights, {})

  -- File lines (1-indexed file index maps to line index - 2)
  for _, file in ipairs(files) do
    local line, highlights = format_file_line(file)
    table.insert(lines, line)
    table.insert(all_highlights, highlights)
  end

  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false

  -- Apply extmark highlights
  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  for i, highlights in ipairs(all_highlights) do
    for _, hl in ipairs(highlights) do
      vim.api.nvim_buf_set_extmark(bufnr, ns, i - 1, hl[1], { end_col = hl[2], hl_group = hl[3] })
    end
  end

  -- Position cursor on selected file (account for header + blank line)
  if selected_index >= 1 and selected_index <= #files then
    local target_line = selected_index + 2 -- header + blank
    if vim.api.nvim_win_is_valid(winnr) then
      vim.api.nvim_win_set_cursor(winnr, { target_line, 0 })
    end
  end
end

return M
