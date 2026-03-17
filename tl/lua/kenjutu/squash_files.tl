local record SquashFileEntry
   path: string
   status: string
   selected: boolean
end

local record JjTreeEntry
   path: string
   status: string
end

local record Highlight
   line_idx: integer
   col_start: integer
   col_end: integer
   hl_group: string
end

local record M
   type SquashFileEntry = SquashFileEntry
   type JjTreeEntry = JjTreeEntry

   open: function(integer, {JjTreeEntry}, function({string}))
end

local ns = vim.api.nvim_create_namespace("kenjutu_squash_files")

local function build_entries(files: {JjTreeEntry}): {SquashFileEntry}
   local entries: {SquashFileEntry} = {}
   for _, file in ipairs(files) do
      table.insert(entries, {
         path = file.path,
         status = file.status,
         selected = true,
      })
   end
   table.sort(entries, function(a: SquashFileEntry, b: SquashFileEntry): boolean
      return a.path < b.path
   end)
   return entries
end

local status_hl: {string: string} = {
   added = "KenjutuStatusA",
   modified = "KenjutuStatusM",
   deleted = "KenjutuStatusD",
   renamed = "KenjutuStatusR",
   copied = "KenjutuStatusC",
   typechange = "KenjutuStatusT",
}

local status_char: {string: string} = {
   added = "A",
   modified = "M",
   deleted = "D",
   renamed = "R",
   copied = "C",
   typechange = "T",
}

local function render(bufnr: integer, entries: {SquashFileEntry})
   local lines: {string} = { " Select files to squash", "" }
   local highlights: {Highlight} = {}

   for i, entry in ipairs(entries) do
      local indicator = entry.selected and "[x]" or "[ ]"
      local sc = status_char[entry.status] or "?"
      local line = " " .. indicator .. "  " .. entry.path .. " " .. sc
      table.insert(lines, line)

      local line_idx = i + 1
      if entry.selected then
         table.insert(highlights, { line_idx = line_idx, col_start = 1, col_end = 4, hl_group = "KenjutuReviewed" })
      end

      local sc_start = #line - #sc
      local hl = status_hl[entry.status] or "KenjutuStats"
      table.insert(highlights, { line_idx = line_idx, col_start = sc_start, col_end = sc_start + #sc, hl_group = hl })
   end

   vim.bo[bufnr].modifiable = true
   vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
   vim.bo[bufnr].modifiable = false

   vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
   table.insert(highlights, { line_idx = 0, col_start = 0, col_end = #lines[1], hl_group = "KenjutuHeader" })
   for _, hl in ipairs(highlights) do
      pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, hl.line_idx, hl.col_start, {
         end_col = hl.col_end,
         hl_group = hl.hl_group,
      } as {string: any})
   end
end

local picker_counter = 0

function M.open(log_winnr: integer, files: {JjTreeEntry}, on_confirm: function({string}))
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
   vim.api.nvim_buf_set_name(bufnr, "squash-files://" .. tostring(picker_counter))

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

   local key_opts: {string: any} = { buffer = bufnr, silent = true }

   vim.keymap.set("n", "<Space>", function()
      local cur = (vim.api.nvim_win_get_cursor(winnr) as {integer})[1]
      local idx = cur - 2
      if idx >= 1 and idx <= #entries then
         entries[idx].selected = not entries[idx].selected
         render(bufnr, entries)
         vim.api.nvim_win_set_cursor(winnr, { cur, 0 })
      end
   end, key_opts)

   vim.keymap.set("n", "<CR>", function()
      local selected: {string} = {}
      for _, entry in ipairs(entries) do
         if entry.selected then
            table.insert(selected, entry.path)
         end
      end
      close_picker()
      on_confirm(selected)
   end, key_opts)

   vim.keymap.set("n", "q", function()
      close_picker()
      on_confirm(nil)
   end, key_opts)

   vim.keymap.set("n", "<Esc>", function()
      close_picker()
      on_confirm(nil)
   end, key_opts)
end

return M
