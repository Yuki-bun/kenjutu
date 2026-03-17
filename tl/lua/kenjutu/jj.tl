local record SystemResult
   code: integer
   stdout: string
   stderr: string
end

local record M
   record Commit
      change_id: string
      commit_id: string
   end

   record HighlightSpan
      col_start: integer
      col_end: integer
      hl_group: string
   end

   record LogResult
      lines: {string}
      highlights: {integer: {HighlightSpan}}
      commits_by_line: {integer: Commit}
      commit_lines: {integer}
   end

   record CommitMetadata
      summary: string
      description: string
      author: string
      timestamp: string
   end

   record SquashOpts
      from: string
      into: string
      paths: {string}
      message: string
   end

   record JjTreeEntry
      path: string
      status: string
   end

   log: function(string, function(string, M.LogResult))
   fetch_commit_metadata: function(string, string, function(string, M.CommitMetadata))
   describe: function(string, string, string, function(string))
   squash: function(string, M.SquashOpts, function(string))
   list_files: function(string, string, function(string, {M.JjTreeEntry}))
   new_commit: function(string, string, function(string))

   _test: {string: any}
end

local TEMPLATE: string = table.concat({
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

local ANSI_256_COLORS: {integer: string} = {
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

local function ansi_256_to_hex(n: integer): string
   if n < 16 then
      return ANSI_256_COLORS[n] or "#808080"
   elseif n < 232 then
      local idx: integer = n - 16
      local b: integer = idx % 6
      idx = math.floor((idx - b) / 6)
      local g: integer = idx % 6
      local r: integer = (idx - g) / 6
      local function to_hex(v: integer): integer
         if v == 0 then
            return 0
         end
         return 55 + v * 40
      end
      return string.format("#%02x%02x%02x", to_hex(r), to_hex(g), to_hex(b))
   else
      local level: integer = 8 + (n - 232) * 10
      return string.format("#%02x%02x%02x", level, level, level)
   end
end

local hl_cache: {string: string} = {}

local function get_hl_group(fg: string, bg: string, bold: boolean): string
   local key: string = (fg or "") .. ":" .. (bg or "") .. ":" .. (bold and "b" or "")
   if hl_cache[key] then
      return hl_cache[key]
   end

   local name: string = "KenjutuAnsi_" .. key:gsub("[#:]", "_")
   local def: {string: any} = {}
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

local function strip_ansi(s: string): string
   local stripped: string = s:gsub("\x1b%[[%d;]*m", "")
   return stripped
end

local function parse_ansi_line(raw: string): string, {M.HighlightSpan}
   local plain_parts: {string} = {}
   local highlights: {M.HighlightSpan} = {}
   local byte_offset: integer = 0

   local cur_fg: string = nil
   local cur_bg: string = nil
   local cur_bold: boolean = false

   local pos: integer = 1
   local len: integer = #raw

   while pos <= len do
      local esc_start: integer = raw:find("\x1b%[", pos, false)
      if not esc_start then
         local text: string = raw:sub(pos)
         if #text > 0 then
            table.insert(plain_parts, text)
            if cur_fg or cur_bg or cur_bold then
               local hl_group: string = get_hl_group(cur_fg, cur_bg, cur_bold)
               table.insert(highlights, {
                  col_start = byte_offset,
                  col_end = byte_offset + #text,
                  hl_group = hl_group,
               } as M.HighlightSpan)
            end
            byte_offset = byte_offset + #text
         end
         break
      end

      if esc_start > pos then
         local text: string = raw:sub(pos, esc_start - 1)
         table.insert(plain_parts, text)
         if cur_fg or cur_bg or cur_bold then
            local hl_group: string = get_hl_group(cur_fg, cur_bg, cur_bold)
            table.insert(highlights, {
               col_start = byte_offset,
               col_end = byte_offset + #text,
               hl_group = hl_group,
            } as M.HighlightSpan)
         end
         byte_offset = byte_offset + #text
      end

      local seq_end: integer = raw:find("m", esc_start + 2, true)
      if not seq_end then
         local text: string = raw:sub(esc_start)
         table.insert(plain_parts, text)
         byte_offset = byte_offset + #text
         break
      end

      local params_str: string = raw:sub(esc_start + 2, seq_end - 1)
      local codes: {integer} = {}
      for c in params_str:gmatch("%d+") do
         table.insert(codes, tonumber(c) as integer)
      end
      if #codes == 0 then
         table.insert(codes, 0)
      end

      local i: integer = 1
      while i <= #codes do
         local c: integer = codes[i]
         if c == 0 then
            cur_fg = nil
            cur_bg = nil
            cur_bold = false
         elseif c == 1 then
            cur_bold = true
         elseif c == 22 then
            cur_bold = false
         elseif c >= 30 and c <= 37 then
            cur_fg = ansi_256_to_hex(c - 30)
         elseif c == 38 then
            if i + 1 <= #codes and codes[i + 1] == 5 and i + 2 <= #codes then
               cur_fg = ansi_256_to_hex(codes[i + 2])
               i = i + 2
            elseif i + 1 <= #codes and codes[i + 1] == 2 and i + 4 <= #codes then
               cur_fg = string.format("#%02x%02x%02x", codes[i + 2], codes[i + 3], codes[i + 4])
               i = i + 4
            end
         elseif c == 39 then
            cur_fg = nil
         elseif c >= 40 and c <= 47 then
            cur_bg = ansi_256_to_hex(c - 40)
         elseif c == 48 then
            if i + 1 <= #codes and codes[i + 1] == 5 and i + 2 <= #codes then
               cur_bg = ansi_256_to_hex(codes[i + 2])
               i = i + 2
            elseif i + 1 <= #codes and codes[i + 1] == 2 and i + 4 <= #codes then
               cur_bg = string.format("#%02x%02x%02x", codes[i + 2], codes[i + 3], codes[i + 4])
               i = i + 4
            end
         elseif c == 49 then
            cur_bg = nil
         elseif c >= 90 and c <= 97 then
            cur_fg = ansi_256_to_hex(c - 90 + 8)
         elseif c >= 100 and c <= 107 then
            cur_bg = ansi_256_to_hex(c - 100 + 8)
         end
         i = i + 1
      end

      pos = seq_end + 1
   end

   return table.concat(plain_parts), highlights
end

function M.log(dir: string, callback: function(string, M.LogResult))
   vim.system(
      { "jj", "log", "--color", "always", "--no-pager", "-T", TEMPLATE },
      { cwd = dir, text = true },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            local err: string = obj.stderr or "jj log failed"
            callback(vim.trim(strip_ansi(err)), nil)
            return
         end

         local stdout: string = obj.stdout or ""
         local raw_lines: {string} = vim.split(stdout, "\n", { plain = true })
         local lines: {string} = {}
         local all_highlights: {integer: {M.HighlightSpan}} = {}
         local commits_by_line: {integer: M.Commit} = {}
         local commit_lines: {integer} = {}

         for _, raw in ipairs(raw_lines) do
            local marker_pos: integer = raw:find("\x01", 1, true)
            if marker_pos then
               local display_raw: string = raw:sub(1, marker_pos - 1)
               local data_raw: string = raw:sub(marker_pos + 1)

               local data_plain: string = strip_ansi(data_raw)
               local fields: {string} = vim.split(data_plain, "\0", { plain = true })
               local change_id: string = fields[1] or ""
               local commit_id: string = fields[2] or ""

               local plain: string, highlights: {M.HighlightSpan} = parse_ansi_line(display_raw)
               table.insert(lines, plain)
               all_highlights[#lines] = highlights
               commits_by_line[#lines] = {
                  change_id = change_id,
                  commit_id = commit_id,
               } as M.Commit
               table.insert(commit_lines, #lines)
            elseif vim.trim(raw) ~= "" then
               local plain: string, highlights: {M.HighlightSpan} = parse_ansi_line(raw)
               table.insert(lines, plain)
               all_highlights[#lines] = highlights
            end
         end

         callback(nil, {
            lines = lines,
            highlights = all_highlights,
            commits_by_line = commits_by_line,
            commit_lines = commit_lines,
         } as M.LogResult)
      end) as function(any)
   )
end

local METADATA_TEMPLATE: string = table.concat({
   "self.description()",
   '++ "\\x00"',
   "++ author.name()",
   '++ "\\x00"',
   "++ author.timestamp().ago()",
}, " ")

function M.fetch_commit_metadata(dir: string, change_id: string, callback: function(string, M.CommitMetadata))
   vim.system(
      { "jj", "log", "-r", change_id, "--no-graph", "--no-pager", "-T", METADATA_TEMPLATE },
      { cwd = dir, text = true },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            callback("jj log metadata failed", nil)
            return
         end
         local stdout: string = vim.trim(obj.stdout or "")
         local fields: {string} = vim.split(stdout, "\0", { plain = true })
         local full_desc: string = vim.trim(fields[1] or "")
         local desc_lines: {string} = vim.split(full_desc, "\n", { plain = true })
         local summary: string = desc_lines[1] or ""
         local body_lines: {string} = {}
         for i = 2, #desc_lines do
            table.insert(body_lines, desc_lines[i])
         end
         local description: string = vim.trim(table.concat(body_lines, "\n"))
         callback(nil, {
            summary = summary,
            description = description,
            author = fields[2] or "",
            timestamp = fields[3] or "",
         } as M.CommitMetadata)
      end) as function(any)
   )
end

function M.describe(dir: string, change_id: string, message: string, callback: function(string))
   vim.system(
      { "jj", "describe", "-r", change_id, "--stdin" },
      { cwd = dir, text = true, stdin = message },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            local err: string = obj.stderr or "jj describe failed"
            callback(vim.trim(strip_ansi(err)))
            return
         end
         callback(nil)
      end) as function(any)
   )
end

function M.squash(dir: string, opts: M.SquashOpts, callback: function(string))
   local cmd: {string} = { "jj", "squash", "--from", opts.from, "--into", opts.into }
   if opts.message then
      table.insert(cmd, "--message")
      table.insert(cmd, opts.message)
   else
      table.insert(cmd, "--use-destination-message")
   end
   if opts.paths then
      for _, path in ipairs(opts.paths) do
         table.insert(cmd, path)
      end
   end
   vim.system(
      cmd,
      { cwd = dir, text = true },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            local err: string = obj.stderr or "jj squash failed"
            callback(vim.trim(strip_ansi(err)))
            return
         end
         callback(nil)
      end) as function(any)
   )
end

local FILES_TEMPLATE: string = [['{"path": ' ++ path.display().escape_json() ++ ', "status": ' ++ status.escape_json() ++ '},']]

function M.list_files(dir: string, change_id: string, callback: function(string, {M.JjTreeEntry}))
   vim.system(
      { "jj", "diff", "-r", change_id, "-T", FILES_TEMPLATE },
      { cwd = dir, text = true },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            local err: string = obj.stderr or "jj diff failed"
            callback(vim.trim(strip_ansi(err)), nil)
            return
         end
         local raw: string = obj.stdout or ""
         if #raw >= 1 and raw:sub(-1) == "," then
            raw = raw:sub(1, -2)
         end
         local output: string = "[" .. raw .. "]"
         local ok: boolean, decoded: any = pcall(vim.fn.json_decode as function(string): any, output)
         if not ok then
            callback("Failed to parse jj diff output: " .. (decoded as string), nil)
            return
         end
         callback(nil, decoded as {M.JjTreeEntry})
      end) as function(any)
   )
end

function M.new_commit(dir: string, change_id: string, callback: function(string))
   vim.system(
      { "jj", "new", "-r", change_id },
      { cwd = dir, text = true },
      vim.schedule_wrap(function(obj_raw: any)
         local obj: SystemResult = obj_raw as SystemResult
         if obj.code ~= 0 then
            local err: string = obj.stderr or "jj new failed"
            callback(vim.trim(strip_ansi(err)))
            return
         end
         callback(nil)
      end) as function(any)
   )
end

M._test = {
   parse_ansi_line = parse_ansi_line,
   ansi_256_to_hex = ansi_256_to_hex,
   strip_ansi = strip_ansi,
} as {string: any}

return M
