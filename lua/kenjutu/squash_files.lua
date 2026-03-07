local utils = require("kenjutu.utils")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_squash_files")

---@class kenjutu.SquashFileEntry
---@field path string
---@field status string
---@field selected boolean

---@param files kenjutu.FileEntry[]
---@return kenjutu.SquashFileEntry[]
local function build_entries(files)
  local entries = {}
  for _, file in ipairs(files) do
    table.insert(entries, {
      path = utils.file_path(file),
      status = file.status,
      selected = true,
    })
  end
  table.sort(entries, function(a, b)
    return a.path < b.path
  end)
  return entries
end

local status_hl = {
  added = "KenjutuStatusA",
  modified = "KenjutuStatusM",
  deleted = "KenjutuStatusD",
  renamed = "KenjutuStatusR",
  copied = "KenjutuStatusC",
  typechange = "KenjutuStatusT",
}

local status_char = {
  added = "A",
  modified = "M",
  deleted = "D",
  renamed = "R",
  copied = "C",
  typechange = "T",
}

---@param bufnr integer
---@param entries kenjutu.SquashFileEntry[]
local function render(bufnr, entries)
  local lines = { " Select files to squash", "" }
  local highlights = {}

  for i, entry in ipairs(entries) do
    local indicator = entry.selected and "[x]" or "[ ]"
    local sc = status_char[entry.status] or "?"
    local line = " " .. indicator .. "  " .. entry.path .. " " .. sc
    table.insert(lines, line)

    local line_idx = i + 1
    if entry.selected then
      table.insert(highlights, { line_idx, 1, 4, "KenjutuReviewed" })
    end

    local sc_start = #line - #sc
    local hl = status_hl[entry.status] or "KenjutuStats"
    table.insert(highlights, { line_idx, sc_start, sc_start + #sc, hl })
  end

  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false

  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  table.insert(highlights, { 0, 0, #lines[1], "KenjutuHeader" })
  for _, hl in ipairs(highlights) do
    pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, hl[1], hl[2], {
      end_col = hl[3],
      hl_group = hl[4],
    })
  end
end

local picker_counter = 0

---@param log_winnr integer
---@param files kenjutu.FileEntry[]
---@param on_confirm fun(paths: string[]|nil)
function M.open(log_winnr, files, on_confirm)
  local entries = build_entries(files)

  vim.api.nvim_set_current_win(log_winnr)
  vim.cmd("aboveleft split")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_win_set_buf(winnr, bufnr)

  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-squash-files"
  picker_counter = picker_counter + 1
  vim.api.nvim_buf_set_name(bufnr, "squash-files://" .. picker_counter)

  vim.wo[winnr].cursorline = true
  vim.wo[winnr].number = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false

  render(bufnr, entries)
  vim.api.nvim_win_set_cursor(winnr, { 3, 0 })

  local function close_picker()
    if vim.api.nvim_win_is_valid(winnr) then
      vim.api.nvim_win_close(winnr, true)
    end
    if vim.api.nvim_buf_is_valid(bufnr) then
      vim.api.nvim_buf_delete(bufnr, { force = true })
    end
  end

  local bopts = { buffer = bufnr, silent = true }

  vim.keymap.set("n", "<Space>", function()
    local cur = vim.api.nvim_win_get_cursor(winnr)[1]
    local idx = cur - 2
    if idx >= 1 and idx <= #entries then
      entries[idx].selected = not entries[idx].selected
      render(bufnr, entries)
      vim.api.nvim_win_set_cursor(winnr, { cur, 0 })
    end
  end, bopts)

  vim.keymap.set("n", "<CR>", function()
    local selected = {}
    for _, entry in ipairs(entries) do
      if entry.selected then
        table.insert(selected, entry.path)
      end
    end
    close_picker()
    on_confirm(selected)
  end, bopts)

  vim.keymap.set("n", "q", function()
    close_picker()
    on_confirm(nil)
  end, bopts)

  vim.keymap.set("n", "<Esc>", function()
    close_picker()
    on_confirm(nil)
  end, bopts)
end

return M
