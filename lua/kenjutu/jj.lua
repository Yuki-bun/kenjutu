local M = {}

---@class kenjutu.Commit
---@field change_id string Full 32-char jj change identifier
---@field commit_id string Full git commit SHA

---@class kenjutu.HighlightSpan
---@field col_start integer byte offset (0-indexed)
---@field col_end integer byte offset (0-indexed, exclusive)
---@field hl_group string Neovim highlight group name

---@class kenjutu.LogResult
---@field lines string[] Display lines for the buffer (1-indexed)
---@field highlights table<integer, kenjutu.HighlightSpan[]> Maps line number to highlight spans
---@field commits_by_line table<integer, kenjutu.Commit> Maps line number to commit data (commit lines only)
---@field commit_lines integer[] Sorted list of line numbers that are commit lines

-- Template that matches jj's default `builtin_log_compact` but appends
-- \x01 + full change_id + \x00 + full commit_id at the end of the header line.
-- This lets us extract full IDs while preserving jj's native colored formatting.
local TEMPLATE = table.concat({
  "if(self.root(),",
  "  format_root_commit(self),",
  "  label(",
  '    separate(" ",',
  '      if(self.current_working_copy(), "working_copy"),',
  '      if(self.immutable(), "immutable", "mutable"),',
  '      if(self.conflict(), "conflicted"),',
  "    ),",
  "    concat(",
  "      format_short_commit_header(self)",
  '        ++ "\\x01" ++ change_id ++ "\\x00" ++ commit_id',
  '        ++ "\\n",',
  '      separate(" ",',
  "        if(self.empty(), empty_commit_marker),",
  "        if(self.description(),",
  "          self.description().first_line(),",
  '          label(if(self.empty(), "empty"), description_placeholder),',
  "        ),",
  '      ) ++ "\\n",',
  "    ),",
  "  )",
  ")",
}, "\n")

local REVSET = "mutable() | ancestors(mutable(), 2)"

-- ANSI 256-color palette to hex mapping for colors 0-15 (standard + bright)
local ANSI_256_COLORS = {
  [0] = "#000000",
  [1] = "#800000",
  [2] = "#008000",
  [3] = "#808000",
  [4] = "#000080",
  [5] = "#800080",
  [6] = "#008080",
  [7] = "#c0c0c0",
  [8] = "#808080",
  [9] = "#ff0000",
  [10] = "#00ff00",
  [11] = "#ffff00",
  [12] = "#0000ff",
  [13] = "#ff00ff",
  [14] = "#00ffff",
  [15] = "#ffffff",
}

--- Convert a 256-color index to a hex color string.
---@param n integer color index (0-255)
---@return string hex color
local function ansi_256_to_hex(n)
  if n < 16 then
    return ANSI_256_COLORS[n] or "#808080"
  elseif n < 232 then
    -- 6x6x6 color cube
    local idx = n - 16
    local b = idx % 6
    idx = (idx - b) / 6
    local g = idx % 6
    local r = (idx - g) / 6
    local function to_hex(v)
      if v == 0 then
        return 0
      end
      return 55 + v * 40
    end
    return string.format("#%02x%02x%02x", to_hex(r), to_hex(g), to_hex(b))
  else
    -- Grayscale ramp
    local level = 8 + (n - 232) * 10
    return string.format("#%02x%02x%02x", level, level, level)
  end
end

-- Cache for dynamically created highlight groups
local hl_cache = {}

--- Get or create a highlight group for a given ANSI style.
---@param fg string|nil foreground hex color
---@param bg string|nil background hex color
---@param bold boolean
---@return string hl_group name
local function get_hl_group(fg, bg, bold)
  local key = (fg or "") .. ":" .. (bg or "") .. ":" .. (bold and "b" or "")
  if hl_cache[key] then
    return hl_cache[key]
  end

  local name = "KenjutuAnsi_" .. key:gsub("[#:]", "_")
  local def = {}
  if fg then
    def.fg = fg
  end
  if bg then
    def.bg = bg
  end
  if bold then
    def.bold = true
  end

  vim.api.nvim_set_hl(0, name, def)
  hl_cache[key] = name
  return name
end

--- Strip ANSI escape codes from a string and return plain text.
---@param s string
---@return string plain text
local function strip_ansi(s)
  local stripped = s:gsub("\x1b%[[%d;]*m", "")
  return stripped
end

--- Parse ANSI escape sequences in a string and produce plain text + highlight spans.
---@param raw string line with ANSI escape codes
---@return string plain plain text (no ANSI codes)
---@return kenjutu.HighlightSpan[] highlights list of highlight spans
local function parse_ansi_line(raw)
  local plain_parts = {}
  local highlights = {}
  local byte_offset = 0

  -- Current style state
  local cur_fg = nil
  local cur_bg = nil
  local cur_bold = false

  local pos = 1
  local len = #raw

  while pos <= len do
    -- Look for the next ESC sequence
    local esc_start = raw:find("\x1b%[", pos, false)
    if not esc_start then
      -- No more escape codes; copy remainder
      local text = raw:sub(pos)
      if #text > 0 then
        table.insert(plain_parts, text)
        if cur_fg or cur_bg or cur_bold then
          local hl_group = get_hl_group(cur_fg, cur_bg, cur_bold)
          table.insert(highlights, {
            col_start = byte_offset,
            col_end = byte_offset + #text,
            hl_group = hl_group,
          })
        end
        byte_offset = byte_offset + #text
      end
      break
    end

    -- Copy text before the escape sequence
    if esc_start > pos then
      local text = raw:sub(pos, esc_start - 1)
      table.insert(plain_parts, text)
      if cur_fg or cur_bg or cur_bold then
        local hl_group = get_hl_group(cur_fg, cur_bg, cur_bold)
        table.insert(highlights, {
          col_start = byte_offset,
          col_end = byte_offset + #text,
          hl_group = hl_group,
        })
      end
      byte_offset = byte_offset + #text
    end

    -- Parse the escape sequence: ESC [ <params> m
    local seq_end = raw:find("m", esc_start + 2, true)
    if not seq_end then
      -- Malformed escape; copy rest as-is
      local text = raw:sub(esc_start)
      table.insert(plain_parts, text)
      byte_offset = byte_offset + #text
      break
    end

    local params_str = raw:sub(esc_start + 2, seq_end - 1)
    local codes = {}
    for c in params_str:gmatch("%d+") do
      table.insert(codes, tonumber(c))
    end
    if #codes == 0 then
      table.insert(codes, 0)
    end

    -- Interpret SGR codes
    local i = 1
    while i <= #codes do
      local c = codes[i]
      if c == 0 then
        -- Reset
        cur_fg = nil
        cur_bg = nil
        cur_bold = false
      elseif c == 1 then
        cur_bold = true
      elseif c == 22 then
        cur_bold = false
      elseif c >= 30 and c <= 37 then
        -- Standard foreground colors (map to 256-color palette indices 0-7)
        cur_fg = ansi_256_to_hex(c - 30)
      elseif c == 38 then
        -- Extended foreground
        if i + 1 <= #codes and codes[i + 1] == 5 and i + 2 <= #codes then
          cur_fg = ansi_256_to_hex(codes[i + 2])
          i = i + 2
        elseif i + 1 <= #codes and codes[i + 1] == 2 and i + 4 <= #codes then
          cur_fg = string.format("#%02x%02x%02x", codes[i + 2], codes[i + 3], codes[i + 4])
          i = i + 4
        end
      elseif c == 39 then
        -- Default foreground
        cur_fg = nil
      elseif c >= 40 and c <= 47 then
        -- Standard background
        cur_bg = ansi_256_to_hex(c - 40)
      elseif c == 48 then
        -- Extended background
        if i + 1 <= #codes and codes[i + 1] == 5 and i + 2 <= #codes then
          cur_bg = ansi_256_to_hex(codes[i + 2])
          i = i + 2
        elseif i + 1 <= #codes and codes[i + 1] == 2 and i + 4 <= #codes then
          cur_bg = string.format("#%02x%02x%02x", codes[i + 2], codes[i + 3], codes[i + 4])
          i = i + 4
        end
      elseif c == 49 then
        -- Default background
        cur_bg = nil
      elseif c >= 90 and c <= 97 then
        -- Bright foreground (map to 256-color palette indices 8-15)
        cur_fg = ansi_256_to_hex(c - 90 + 8)
      elseif c >= 100 and c <= 107 then
        -- Bright background
        cur_bg = ansi_256_to_hex(c - 100 + 8)
      end
      i = i + 1
    end

    pos = seq_end + 1
  end

  return table.concat(plain_parts), highlights
end

--- Run `jj log` asynchronously and parse the output.
--- Produces colored output using jj's native formatting with ANSI codes parsed
--- into Neovim highlight spans.
---@param dir string working directory
---@param callback fun(err: string|nil, result: kenjutu.LogResult|nil)
function M.log(dir, callback)
  vim.system(
    { "jj", "log", "--color", "always", "--no-pager", "-r", REVSET, "-T", TEMPLATE },
    { cwd = dir, text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        local err = obj.stderr or "jj log failed"
        -- Strip ANSI from error messages
        callback(vim.trim(strip_ansi(err)), nil)
        return
      end

      local stdout = obj.stdout or ""
      local raw_lines = vim.split(stdout, "\n", { plain = true })
      local lines = {}
      local all_highlights = {}
      local commits_by_line = {}
      local commit_lines = {}

      for _, raw in ipairs(raw_lines) do
        -- Check for our \x01 marker (commit header lines)
        local marker_pos = raw:find("\x01", 1, true)
        if marker_pos then
          -- Split: display portion (before \x01) and data portion (after \x01)
          local display_raw = raw:sub(1, marker_pos - 1)
          local data_raw = raw:sub(marker_pos + 1)

          -- Extract full IDs from the data portion (strip ANSI first)
          local data_plain = strip_ansi(data_raw)
          local fields = vim.split(data_plain, "\0", { plain = true })
          local change_id = fields[1] or ""
          local commit_id = fields[2] or ""

          -- Parse ANSI codes from the display portion
          local plain, highlights = parse_ansi_line(display_raw)
          table.insert(lines, plain)
          all_highlights[#lines] = highlights
          commits_by_line[#lines] = {
            change_id = change_id,
            commit_id = commit_id,
          }
          table.insert(commit_lines, #lines)
        elseif vim.trim(raw) ~= "" then
          -- Non-commit lines (description, graph continuation, etc.)
          local plain, highlights = parse_ansi_line(raw)
          table.insert(lines, plain)
          all_highlights[#lines] = highlights
        end
      end

      callback(nil, {
        lines = lines,
        highlights = all_highlights,
        commits_by_line = commits_by_line,
        commit_lines = commit_lines,
      })
    end)
  )
end

return M
