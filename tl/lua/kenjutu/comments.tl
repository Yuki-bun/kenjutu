local kjn = require("kenjutu.kjn")

local record M
   record FloatInputOpts
      title: string
      initial_body: string
      on_save: function(string)
      on_cancel: function()
   end

   record OpenThreadOpts
      file_path: string
      line: integer
      side: string
      comments: {kjn.PortedComment}
   end

   record OpenNewCommentFloatOpts
      dir: string
      file_path: string
      change_id: string
      commit_id: string
      on_create: function()
      side: string
   end

   record BuildCommentListResult
      lines: {string}
      fold_levels: {integer: string}
      highlights: {{string:any}}
      line_to_comment: {integer: kjn.PortedComment}
      comment_fold_ranges: {CommentFoldRange}
   end

   record CommentFoldRange
      fold_start_line: integer
      resolved: boolean
   end

   record OpenCommentListOpts
      file_path: string
      comments: {kjn.PortedComment}
      on_select: function(kjn.PortedComment)
      dir: string
      change_id: string
      on_resolve: function()
   end

   comments_at_line: function({kjn.PortedComment}, integer, string): {kjn.PortedComment}
   open_thread: function(M.OpenThreadOpts): integer, integer
   open_new_comment: function(M.OpenNewCommentFloatOpts): integer, integer
   build_comment_list: function({kjn.PortedComment}, integer): M.BuildCommentListResult
   open_comment_list: function(M.OpenCommentListOpts): integer, integer
   place_signs: function(integer, {kjn.PortedComment}, string)
   goto_next_comment: function()
   goto_prev_comment: function()
end

vim.api.nvim_set_hl(0, "KenjutuCommentSign", { default = true, fg = "#90ee90" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentSignResolved", { default = true, fg = "#4a6e4a" } as {string:any})

vim.fn.sign_define("KenjutuComment", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSign" })
vim.fn.sign_define("KenjutuCommentResolved", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSignResolved" })

vim.api.nvim_set_hl(0, "KenjutuCommentTimestamp", { default = true, link = "Comment" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentSeparator", { default = true, link = "Comment" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentResolved", { default = true, link = "DiagnosticOk" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentHeader", { default = true, link = "Title" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentCodeSnippet", { default = true, link = "Comment" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentCodeGutter", { default = true, link = "LineNr" } as {string:any})
vim.api.nvim_set_hl(0, "KenjutuCommentReplyHeader", { default = true, link = "Comment" } as {string:any})

local NS: integer = vim.api.nvim_create_namespace("kenjutu_comment_thread")

global function kenjutu_comment_list_foldexpr(lnum: integer): string
   local levels: {integer: string} = vim.b[0].kenjutu_fold_levels as {integer: string}
   if levels then
      return levels[lnum] or "0"
   end
   return "0"
end

global function kenjutu_comment_list_foldtext(): string
   local line: string = vim.fn.getline(vim.v.foldstart)
   local count: integer = vim.v.foldend - vim.v.foldstart + 1
   if vim.v.foldlevel == 2 then
      return line
   end
   return line .. " (" .. count .. " lines)"
end

local function open_float_input(opts: M.FloatInputOpts): integer, integer
   local buf: integer = vim.api.nvim_create_buf(false, true)
   vim.bo[buf].buftype = "acwrite"
   vim.bo[buf].swapfile = false
   vim.bo[buf].buflisted = false
   vim.bo[buf].filetype = "kenjutu-comment-input"

   local initial_lines: {string} = { "" }
   if opts.initial_body and opts.initial_body ~= "" then
      initial_lines = vim.split(opts.initial_body, "\n", { plain = true })
   end
   vim.api.nvim_buf_set_lines(buf, 0, -1, false, initial_lines)

   local width: integer = math.min(100, math.floor(vim.o.columns * 0.6))
   local height: integer = math.max(3, math.min(10, #initial_lines + 2))

   local win: integer = vim.api.nvim_open_win(buf, true, {
      relative = "cursor",
      row = 1,
      col = 0,
      width = width,
      height = height,
      style = "minimal",
      border = "rounded",
      title = " " .. opts.title .. " ",
      title_pos = "center",
   } as {string:any})

   vim.wo[win].wrap = true
   vim.wo[win].cursorline = false

   pcall(vim.api.nvim_buf_set_name, buf, "kjn://comment-input")

   local closed: boolean = false
   local function close_float()
      if closed then
         return
      end
      closed = true
      if vim.api.nvim_win_is_valid(win) then
         vim.api.nvim_win_close(win, true)
      end
      if vim.api.nvim_buf_is_valid(buf) then
         vim.bo[buf].modified = false
         vim.api.nvim_buf_delete(buf, { force = true } as {string:any})
      end
   end

   vim.api.nvim_create_autocmd("BufWriteCmd", {
      buffer = buf,
      callback = function()
         local lines: {string} = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
         local body: string = vim.fn.trim(table.concat(lines, "\n"))
         if body == "" then
            vim.notify("Comment body cannot be empty", vim.log.levels.WARN)
            return
         end
         vim.bo[buf].modified = false
         close_float()
         opts.on_save(body)
      end,
   } as {string:any})

   vim.keymap.set("n", "q", function()
      close_float()
      if opts.on_cancel then
         opts.on_cancel()
      end
   end, { buffer = buf, silent = true, nowait = true } as {string:any})

   vim.api.nvim_create_autocmd("WinClosed", {
      buffer = buf,
      once = true,
      callback = function()
         if not closed then
            closed = true
            if opts.on_cancel then
               opts.on_cancel()
            end
         end
      end,
   } as {string:any})

   vim.cmd("startinsert")

   return buf, win
end

local function comment_signs(bufnr: integer): {integer}
   local placed: {{string:any}} = vim.fn.sign_getplaced(bufnr, { group = "kenjutu_comments" })
   if #placed == 0 then
      return {}
   end
   local first: {string:any} = placed[1]
   local signs: {{string:any}} = first.signs as {{string:any}}
   if not signs or #signs == 0 then
      return {}
   end
   table.sort(signs, function(a: {string:any}, b: {string:any}): boolean
      return (a.lnum as integer) < (b.lnum as integer)
   end)
   local lines: {integer} = {}
   for _, sign in ipairs(signs) do
      table.insert(lines, sign.lnum as integer)
   end
   return lines
end

function M.comments_at_line(file_comments: {kjn.PortedComment}, line: integer, side_filter: string): {kjn.PortedComment}
   local result: {kjn.PortedComment} = {}
   for _, pc in ipairs(file_comments) do
      local ported: integer = pc.ported_line
      if ported and (not side_filter or pc.comment.side == side_filter) then
         local start: integer = pc.ported_start_line or ported
         if line >= start and line <= ported then
            table.insert(result, pc)
         end
      end
   end
   return result
end

local function format_date(date_str: string): string
   local y: string, m: string, d: string = date_str:match("^(%d%d%d%d)-(%d%d)-(%d%d)")
   if y then
      return y .. "-" .. m .. "-" .. d
   end
   return date_str
end

function M.open_thread(opts: M.OpenThreadOpts): integer, integer
   if #opts.comments == 0 then
      return nil, nil
   end

   local width: integer = math.floor(vim.o.columns * 0.6)
   local separator: string = string.rep("\xe2\x94\x80", width)
   local double_separator: string = string.rep("\xe2\x95\x90", width)
   local lines: {string} = {}
   local highlights: {{string:any}} = {}

   for i, pc in ipairs(opts.comments) do
      if i > 1 then
         table.insert(lines, "")
         table.insert(lines, double_separator)
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentSeparator" } as {string:any})
         table.insert(lines, "")
      end

      local comment: kjn.MaterializedComment = pc.comment
      for _, body_line in ipairs(vim.split(comment.body, "\n", { plain = true })) do
         table.insert(lines, body_line)
      end
      local date: string = format_date(comment.created_at)
      table.insert(lines, string.rep(" ", math.max(0, width - #date - 2)) .. date)
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" } as {string:any})

      local replies: {kjn.MaterializedReply} = comment.replies or ({} as {kjn.MaterializedReply})
      for _, reply in ipairs(replies) do
         table.insert(lines, separator)
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentSeparator" } as {string:any})
         for _, body_line in ipairs(vim.split(reply.body, "\n", { plain = true })) do
            table.insert(lines, "  " .. body_line)
         end
         local reply_date: string = format_date(reply.created_at)
         table.insert(lines, string.rep(" ", math.max(0, width - #reply_date - 2)) .. reply_date)
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" } as {string:any})
      end
   end

   local buf: integer = vim.api.nvim_create_buf(false, true)
   vim.bo[buf].buftype = "nofile"
   vim.bo[buf].swapfile = false
   vim.bo[buf].buflisted = false
   vim.bo[buf].modifiable = true
   vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
   vim.bo[buf].modifiable = false

   for _, hl in ipairs(highlights) do
      vim.api.nvim_buf_set_extmark(buf, NS, hl.line as integer, 0, {
         end_col = #lines[(hl.line as integer) + 1],
         hl_group = hl.hl,
      } as {string:any})
   end

   local height: integer = math.min(#lines, math.floor(vim.o.lines * 0.7))

   local resolved: boolean = opts.comments[1].comment.resolved
   local title_parts: {string} = { "Thread: ", opts.file_path, ":L", tostring(opts.line), " (", opts.side, ")" }
   if resolved then
      table.insert(title_parts, " [resolved]")
   end
   local title: string = table.concat(title_parts)

   local win: integer = vim.api.nvim_open_win(buf, true, {
      relative = "cursor",
      row = 1,
      col = 0,
      width = width,
      height = height,
      style = "minimal",
      border = "rounded",
      title = " " .. title .. " ",
      title_pos = "center",
   } as {string:any})

   vim.wo[win].wrap = true
   vim.wo[win].cursorline = false

   local closed: boolean = false
   local function close_float()
      if closed then
         return
      end
      closed = true
      if vim.api.nvim_win_is_valid(win) then
         vim.api.nvim_win_close(win, true)
      end
      if vim.api.nvim_buf_is_valid(buf) then
         vim.api.nvim_buf_delete(buf, { force = true } as {string:any})
      end
   end

   vim.keymap.set("n", "q", close_float, { buffer = buf, silent = true, nowait = true } as {string:any})
   vim.keymap.set("n", "<Esc>", close_float, { buffer = buf, silent = true, nowait = true } as {string:any})

   vim.api.nvim_create_autocmd("WinClosed", {
      buffer = buf,
      once = true,
      callback = function()
         closed = true
      end,
   } as {string:any})

   return buf, win
end

function M.open_new_comment(opts: M.OpenNewCommentFloatOpts): integer, integer
   local is_visual: boolean = vim.fn.mode() == "v" or vim.fn.mode() == "V" or vim.fn.mode() == "\22"
   local cursor_line: integer = vim.api.nvim_win_get_cursor(0)[1]
   local anchor_line: integer = vim.fn.line("v")

   local record LineOpts
      line: integer
      start_line: integer
   end

   local line_opts: LineOpts
   if is_visual then
      line_opts = {
         line = math.max(cursor_line, anchor_line),
         start_line = math.min(cursor_line, anchor_line),
      }
   else
      line_opts = {
         line = cursor_line,
      }
   end

   local title: string = string.format("New comment %s:L%d (%s)", opts.file_path, line_opts.line, opts.side)

   local float_bufnr: integer, float_winnr: integer = open_float_input({
      title = title,
      on_save = function(body: string)
         kjn.add_comment({
            dir = opts.dir,
            change_id = opts.change_id,
            commit_id = opts.commit_id,
            file_path = opts.file_path,
            side = opts.side,
            line = line_opts.line,
            start_line = line_opts.start_line,
            body = body,
         }, function(err: string, _: {string:any})
            if err then
               vim.notify("add comment: " .. err, vim.log.levels.ERROR)
               return
            end
            opts.on_create()
         end)
      end,
   } as M.FloatInputOpts)

   return float_bufnr, float_winnr
end

function M.build_comment_list(comments: {kjn.PortedComment}, width: integer): M.BuildCommentListResult
   local lines: {string} = {}
   local fold_levels: {integer: string} = {}
   local highlights: {{string:any}} = {}
   local line_to_comment: {integer: kjn.PortedComment} = {}
   local comment_fold_ranges: {M.CommentFoldRange} = {}

   for i, pc in ipairs(comments) do
      if i > 1 then
         table.insert(lines, "")
         fold_levels[#lines] = "0"
      end

      local c: kjn.MaterializedComment = pc.comment
      local line_num: string = pc.ported_line and ("L" .. pc.ported_line) or "L?"
      local side: string = c.side or "?"
      local resolved_tag: string = c.resolved and " [resolved]" or ""
      local date: string = format_date(c.created_at)
      local header_prefix: string = line_num .. " (" .. side .. ")" .. resolved_tag
      local header: string = header_prefix .. string.rep(" ", math.max(1, width - #header_prefix - #date)) .. date

      table.insert(lines, header)
      fold_levels[#lines] = "0"
      line_to_comment[#lines] = pc
      if c.resolved then
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentResolved" } as {string:any})
      else
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentHeader" } as {string:any})
      end
      table.insert(
         highlights,
         { line = #lines - 1, col = #header - #date, end_col = #header, hl = "KenjutuCommentTimestamp" } as {string:any}
      )

      local body_lines: {string} = vim.split(c.body, "\n", { plain = true })
      for j, body_line in ipairs(body_lines) do
         table.insert(lines, "  " .. body_line)
         fold_levels[#lines] = j == 1 and ">1" or "1"
         line_to_comment[#lines] = pc
      end
      table.insert(comment_fold_ranges, { fold_start_line = #lines - #body_lines + 1, resolved = c.resolved } as M.CommentFoldRange)

      local anchor: kjn.Anchor = c.anchor
      local target: {string} = anchor and anchor.target or ({} as {string})
      for j, code_line in ipairs(target) do
         local rendered: string = "  \xe2\x94\x82 " .. code_line
         table.insert(lines, rendered)
         fold_levels[#lines] = j == 1 and ">2" or "2"
         line_to_comment[#lines] = pc
         table.insert(highlights, { line = #lines - 1, col = 2, end_col = 5, hl = "KenjutuCommentCodeGutter" } as {string:any})
         table.insert(highlights, { line = #lines - 1, col = 5, end_col = #rendered, hl = "KenjutuCommentCodeSnippet" } as {string:any})
      end

      local reply_list: {kjn.MaterializedReply} = c.replies or ({} as {kjn.MaterializedReply})
      for _, reply in ipairs(reply_list) do
         local reply_sep: string = "  "
            .. string.rep("\xe2\x94\x80", math.max(1, width - 12))
            .. " Reply "
            .. string.rep("\xe2\x94\x80", 3)
         table.insert(lines, reply_sep)
         fold_levels[#lines] = "1"
         line_to_comment[#lines] = pc
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentReplyHeader" } as {string:any})

         for _, body_line in ipairs(vim.split(reply.body, "\n", { plain = true })) do
            table.insert(lines, "    " .. body_line)
            fold_levels[#lines] = "1"
            line_to_comment[#lines] = pc
         end

         local reply_date: string = format_date(reply.created_at)
         table.insert(lines, string.rep(" ", math.max(0, width - #reply_date)) .. reply_date)
         fold_levels[#lines] = "1"
         line_to_comment[#lines] = pc
         table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" } as {string:any})
      end
   end

   return {
      lines = lines,
      fold_levels = fold_levels,
      highlights = highlights,
      line_to_comment = line_to_comment,
      comment_fold_ranges = comment_fold_ranges,
   } as M.BuildCommentListResult
end

local function render_comment_list(result: M.BuildCommentListResult, buf: integer, win: integer)
   vim.bo[buf].modifiable = true
   vim.api.nvim_buf_set_lines(buf, 0, -1, false, result.lines)
   vim.bo[buf].modifiable = false

   vim.api.nvim_buf_clear_namespace(buf, NS, 0, -1)
   for _, hl in ipairs(result.highlights) do
      local col: integer = (hl.col or 0) as integer
      local end_col: integer = (hl.end_col or #result.lines[(hl.line as integer) + 1]) as integer
      vim.api.nvim_buf_set_extmark(buf, NS, hl.line as integer, col, {
         end_col = end_col,
         hl_group = hl.hl,
      } as {string:any})
   end

   vim.b[buf].kenjutu_fold_levels = result.fold_levels as any

   if vim.api.nvim_win_is_valid(win) then
      vim.cmd("normal! zM")
      for _, entry in ipairs(result.comment_fold_ranges) do
         if not entry.resolved then
            vim.api.nvim_win_set_cursor(win, { entry.fold_start_line, 0 })
            vim.cmd("normal! zo")
         end
      end
   end
end

function M.open_comment_list(opts: M.OpenCommentListOpts): integer, integer
   local comments: {kjn.PortedComment} = {}
   for _, pc in ipairs(opts.comments) do
      table.insert(comments, pc)
   end
   table.sort(comments, function(a: kjn.PortedComment, b: kjn.PortedComment): boolean
      local la: number = a.ported_line or math.huge
      local lb: number = b.ported_line or math.huge
      return la < lb
   end)

   if #comments == 0 then
      vim.notify("No comments on this file", vim.log.levels.INFO)
      return nil, nil
   end

   local width: integer = math.floor(vim.o.columns * 0.6)
   local result: M.BuildCommentListResult = M.build_comment_list(comments, width)

   local buf: integer = vim.api.nvim_create_buf(false, true)
   vim.bo[buf].buftype = "nofile"
   vim.bo[buf].swapfile = false
   vim.bo[buf].buflisted = false
   vim.bo[buf].modifiable = true
   vim.api.nvim_buf_set_lines(buf, 0, -1, false, result.lines)
   vim.bo[buf].modifiable = false

   for _, hl in ipairs(result.highlights) do
      local col: integer = (hl.col or 0) as integer
      local end_col: integer = (hl.end_col or #result.lines[(hl.line as integer) + 1]) as integer
      vim.api.nvim_buf_set_extmark(buf, NS, hl.line as integer, col, {
         end_col = end_col,
         hl_group = hl.hl,
      } as {string:any})
   end

   vim.b[buf].kenjutu_fold_levels = result.fold_levels as any

   local height: integer = math.min(#result.lines, math.floor(vim.o.lines * 0.7))
   local title: string = "Comments: " .. opts.file_path

   local win: integer = vim.api.nvim_open_win(buf, true, {
      relative = "editor",
      row = math.floor((vim.o.lines - height) / 2),
      col = math.floor((vim.o.columns - width) / 2),
      width = width,
      height = height,
      style = "minimal",
      border = "rounded",
      title = " " .. title .. " ",
      title_pos = "center",
   } as {string:any})

   vim.wo[win].wrap = true
   vim.wo[win].cursorline = true
   vim.wo[win].foldenable = true
   vim.wo[win].foldmethod = "expr"
   vim.wo[win].foldexpr = "v:lua.kenjutu_comment_list_foldexpr(v:lnum)"
   vim.wo[win].foldtext = "v:lua.kenjutu_comment_list_foldtext()"
   vim.wo[win].foldcolumn = "0"
   vim.wo[win].fillchars = "fold: "

   vim.cmd("normal! zM")
   for _, entry in ipairs(result.comment_fold_ranges) do
      if not entry.resolved then
         vim.api.nvim_win_set_cursor(win, { entry.fold_start_line, 0 })
         vim.cmd("normal! zo")
      end
   end
   vim.api.nvim_win_set_cursor(win, { 1, 0 })

   local closed: boolean = false
   local function close_float()
      if closed then
         return
      end
      closed = true
      if vim.api.nvim_win_is_valid(win) then
         vim.api.nvim_win_close(win, true)
      end
      if vim.api.nvim_buf_is_valid(buf) then
         vim.api.nvim_buf_delete(buf, { force = true } as {string:any})
      end
   end

   local function selected_comment(): kjn.PortedComment
      local cursor_line: integer = vim.api.nvim_win_get_cursor(win)[1]
      return result.line_to_comment[cursor_line]
   end

   vim.keymap.set("n", "q", close_float, { buffer = buf, silent = true, nowait = true } as {string:any})
   vim.keymap.set("n", "<Esc>", close_float, { buffer = buf, silent = true, nowait = true } as {string:any})

   vim.keymap.set("n", "<CR>", function()
      local pc: kjn.PortedComment = selected_comment()
      if pc then
         close_float()
         opts.on_select(pc)
      end
   end, { buffer = buf, silent = true, nowait = true } as {string:any})

   vim.keymap.set("n", "x", function()
      local pc: kjn.PortedComment = selected_comment()
      if not pc then
         return
      end
      local resolve_fn: function(kjn.ResolveCommentOptions, function(string, {string:any}))
      if pc.comment.resolved then
         resolve_fn = kjn.unresolve_comment
      else
         resolve_fn = kjn.resolve_comment
      end
      resolve_fn({
         dir = opts.dir,
         change_id = opts.change_id,
         file_path = opts.file_path,
         comment_id = pc.comment.id,
      }, function(err: string, _: {string:any})
         if err then
            vim.notify("resolve comment: " .. err, vim.log.levels.ERROR)
            return
         end
         pc.comment.resolved = not pc.comment.resolved
         local cursor_line: integer = vim.api.nvim_win_get_cursor(win)[1]
         result = M.build_comment_list(comments, width)
         render_comment_list(result, buf, win)
         local new_cursor: integer = math.min(cursor_line, #result.lines)
         vim.api.nvim_win_set_cursor(win, { new_cursor, 0 })
         if opts.on_resolve then
            opts.on_resolve()
         end
      end)
   end, { buffer = buf, silent = true, nowait = true } as {string:any})

   vim.api.nvim_create_autocmd("WinClosed", {
      buffer = buf,
      once = true,
      callback = function()
         closed = true
      end,
   } as {string:any})

   return buf, win
end

function M.place_signs(bufnr: integer, file_comments: {kjn.PortedComment}, side_filter: string)
   vim.fn.sign_unplace("kenjutu_comments", { buffer = bufnr })
   for _, pc in ipairs(file_comments) do
      if pc.ported_line and (not side_filter or pc.comment.side == side_filter) then
         local sign_name: string = pc.comment.resolved and "KenjutuCommentResolved" or "KenjutuComment"
         vim.fn.sign_place(0, "kenjutu_comments", sign_name, bufnr, { lnum = pc.ported_line } as {string:any})
      end
   end
end

function M.goto_next_comment()
   local bufnr: integer = vim.api.nvim_get_current_buf()
   local cursor_line: integer = vim.api.nvim_win_get_cursor(0)[1]
   local signs: {integer} = comment_signs(bufnr)
   for _, line in ipairs(signs) do
      if line > cursor_line then
         vim.api.nvim_win_set_cursor(0, { line, 0 })
         return
      end
   end
end

function M.goto_prev_comment()
   local bufnr: integer = vim.api.nvim_get_current_buf()
   local cursor_line: integer = vim.api.nvim_win_get_cursor(0)[1]
   local signs: {integer} = comment_signs(bufnr)
   for i = #signs, 1, -1 do
      if signs[i] < cursor_line then
         vim.api.nvim_win_set_cursor(0, { signs[i], 0 })
         return
      end
   end
end

return M
