local kjn = require("kenjutu.kjn")

local M = {}

vim.api.nvim_set_hl(0, "KenjutuCommentSign", { default = true, link = "DiagnosticSignOk" })
vim.api.nvim_set_hl(0, "KenjutuCommentSignResolved", { default = true, link = "NonText" })

vim.fn.sign_define("KenjutuComment", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSign" })
vim.fn.sign_define("KenjutuCommentResolved", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSignResolved" })

vim.api.nvim_set_hl(0, "KenjutuCommentTimestamp", { default = true, link = "Comment" })
vim.api.nvim_set_hl(0, "KenjutuCommentSeparator", { default = true, link = "Comment" })
vim.api.nvim_set_hl(0, "KenjutuCommentResolved", { default = true, link = "DiagnosticOk" })
vim.api.nvim_set_hl(0, "KenjutuCommentHeader", { default = true, link = "Title" })
vim.api.nvim_set_hl(0, "KenjutuCommentCodeSnippet", { default = true, link = "Comment" })
vim.api.nvim_set_hl(0, "KenjutuCommentCodeGutter", { default = true, link = "LineNr" })
vim.api.nvim_set_hl(0, "KenjutuCommentReplyHeader", { default = true, link = "Comment" })

local NS = vim.api.nvim_create_namespace("kenjutu_comment_thread")

function _G.kenjutu_comment_list_foldexpr(lnum)
  local levels = vim.b[0].kenjutu_fold_levels
  if levels then
    return levels[lnum] or "0"
  end
  return "0"
end

function _G.kenjutu_comment_list_foldtext()
  local line = vim.fn.getline(vim.v.foldstart)
  local count = vim.v.foldend - vim.v.foldstart + 1
  if vim.v.foldlevel == 2 then
    return line
  end
  return line .. " (" .. count .. " lines)"
end

---@class kenjutu.FloatInputOpts
---@field title string
---@field initial_body string|nil  pre-fill for edit mode
---@field on_save fun(body: string)  called with trimmed body on :w
---@field on_cancel fun()|nil  called on q or closing the float

---@param opts kenjutu.FloatInputOpts
---@return integer bufnr
---@return integer winnr
local function open_float_input(opts)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "acwrite"
  vim.bo[buf].swapfile = false
  vim.bo[buf].buflisted = false
  vim.bo[buf].filetype = "kenjutu-comment-input"

  local initial_lines = { "" }
  if opts.initial_body and opts.initial_body ~= "" then
    initial_lines = vim.split(opts.initial_body, "\n", { plain = true })
  end
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, initial_lines)

  local width = math.min(100, math.floor(vim.o.columns * 0.6))
  local height = math.max(3, math.min(10, #initial_lines + 2))

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "cursor",
    row = 1,
    col = 0,
    width = width,
    height = height,
    style = "minimal",
    border = "rounded",
    title = " " .. opts.title .. " ",
    title_pos = "center",
  })

  vim.wo[win].wrap = true
  vim.wo[win].cursorline = false

  pcall(vim.api.nvim_buf_set_name, buf, "kjn://comment-input")

  local closed = false
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
      vim.api.nvim_buf_delete(buf, { force = true })
    end
  end

  vim.api.nvim_create_autocmd("BufWriteCmd", {
    buffer = buf,
    callback = function()
      local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
      local body = vim.fn.trim(table.concat(lines, "\n"))
      if body == "" then
        vim.notify("Comment body cannot be empty", vim.log.levels.WARN)
        return
      end
      vim.bo[buf].modified = false
      close_float()
      opts.on_save(body)
    end,
  })

  vim.keymap.set("n", "q", function()
    close_float()
    if opts.on_cancel then
      opts.on_cancel()
    end
  end, { buffer = buf, silent = true, nowait = true })

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
  })

  vim.cmd("startinsert")

  return buf, win
end

---@param bufnr integer
---@return integer[] line numbers where signs were placed
local function comment_signs(bufnr)
  local placed = vim.fn.sign_getplaced(bufnr, { group = "kenjutu_comments" })
  if #placed == 0 or #placed[1].signs == 0 then
    return {}
  end
  local signs = placed[1].signs
  table.sort(signs, function(a, b)
    return a.lnum < b.lnum
  end)
  local lines = {}
  for _, sign in ipairs(signs) do
    table.insert(lines, sign.lnum)
  end
  return lines
end

---@param file_comments kenjutu.PortedComment[]
---@param line integer
---@param side_filter "Old"|"New"|nil
---@return kenjutu.PortedComment[]
function M.comments_at_line(file_comments, line, side_filter)
  local result = {}
  for _, pc in ipairs(file_comments) do
    local ported = pc.ported_line
    if ported and (not side_filter or pc.comment.side == side_filter) then
      local start = pc.ported_start_line or ported
      if line >= start and line <= ported then
        table.insert(result, pc)
      end
    end
  end
  return result
end

---@param date_str string
---@return string
local function format_date(date_str)
  local y, m, d = date_str:match("^(%d%d%d%d)-(%d%d)-(%d%d)")
  if y then
    return y .. "-" .. m .. "-" .. d
  end
  return date_str
end

---@class kenjutu.OpenThreadOpts
---@field file_path string
---@field line integer
---@field side "Old"|"New"
---@field comments kenjutu.PortedComment[]

---@param opts kenjutu.OpenThreadOpts
---@return integer|nil bufnr
---@return integer|nil winnr
function M.open_thread(opts)
  if #opts.comments == 0 then
    return nil, nil
  end

  local width = math.floor(vim.o.columns * 0.6)
  local separator = string.rep("─", width)
  local double_separator = string.rep("═", width)
  local lines = {}
  ---@type { line: integer, hl: string }[]
  local highlights = {}

  for i, pc in ipairs(opts.comments) do
    if i > 1 then
      table.insert(lines, "")
      table.insert(lines, double_separator)
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentSeparator" })
      table.insert(lines, "")
    end

    local comment = pc.comment
    for _, body_line in ipairs(vim.split(comment.body, "\n", { plain = true })) do
      table.insert(lines, body_line)
    end
    local date = format_date(comment.created_at)
    table.insert(lines, string.rep(" ", math.max(0, width - #date - 2)) .. date)
    table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" })

    for _, reply in ipairs(comment.replies or {}) do
      table.insert(lines, separator)
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentSeparator" })
      for _, body_line in ipairs(vim.split(reply.body, "\n", { plain = true })) do
        table.insert(lines, "  " .. body_line)
      end
      local reply_date = format_date(reply.created_at)
      table.insert(lines, string.rep(" ", math.max(0, width - #reply_date - 2)) .. reply_date)
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" })
    end
  end

  local buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].swapfile = false
  vim.bo[buf].buflisted = false
  vim.bo[buf].modifiable = true
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false

  for _, hl in ipairs(highlights) do
    vim.api.nvim_buf_set_extmark(buf, NS, hl.line, 0, {
      end_col = #lines[hl.line + 1],
      hl_group = hl.hl,
    })
  end

  local height = math.min(#lines, math.floor(vim.o.lines * 0.7))

  local resolved = opts.comments[1].comment.resolved
  local title_parts = { "Thread: ", opts.file_path, ":L", tostring(opts.line), " (", opts.side, ")" }
  if resolved then
    table.insert(title_parts, " [resolved]")
  end
  local title = table.concat(title_parts)

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "cursor",
    row = 1,
    col = 0,
    width = width,
    height = height,
    style = "minimal",
    border = "rounded",
    title = " " .. title .. " ",
    title_pos = "center",
  })

  vim.wo[win].wrap = true
  vim.wo[win].cursorline = false

  local closed = false
  local function close_float()
    if closed then
      return
    end
    closed = true
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
    if vim.api.nvim_buf_is_valid(buf) then
      vim.api.nvim_buf_delete(buf, { force = true })
    end
  end

  vim.keymap.set("n", "q", close_float, { buffer = buf, silent = true, nowait = true })
  vim.keymap.set("n", "<Esc>", close_float, { buffer = buf, silent = true, nowait = true })

  vim.api.nvim_create_autocmd("WinClosed", {
    buffer = buf,
    once = true,
    callback = function()
      closed = true
    end,
  })

  return buf, win
end

---@class kenjutu.OpenNewCommentFloatOpts
---@field dir string
---@field file_path string
---@field change_id string
---@field commit_id string
---@field on_create fun()
---@field side "Old"|"New"

--- Open a floating input window for creating a new comment.
--- Does not create a thread buffer — the float stands alone.
---@param opts kenjutu.OpenNewCommentFloatOpts
---@return integer float_bufnr
---@return integer float_winnr
function M.open_new_comment(opts)
  local is_visual = vim.fn.mode() == "v" or vim.fn.mode() == "V" or vim.fn.mode() == "\22"
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local anchor_line = vim.fn.line("v")
  local line_opts = is_visual
      and {
        line = math.max(cursor_line, anchor_line),
        start_line = math.min(cursor_line, anchor_line),
      }
    or {
      line = cursor_line,
      start_line = nil,
    }

  local title = string.format("New comment %s:L%d (%s)", opts.file_path, line_opts.line, opts.side)

  local float_bufnr, float_winnr = open_float_input({
    title = title,
    on_save = function(body)
      kjn.add_comment({
        dir = opts.dir,
        change_id = opts.change_id,
        commit_id = opts.commit_id,
        file_path = opts.file_path,
        side = opts.side,
        line = line_opts.line,
        start_line = line_opts.start_line,
        body = body,
      }, function(err, _)
        if err then
          vim.notify("add comment: " .. err, vim.log.levels.ERROR)
          return
        end
        opts.on_create()
      end)
    end,
  })

  return float_bufnr, float_winnr
end

---@class kenjutu.BuildCommentListResult
---@field lines string[]
---@field fold_levels table<integer, string>
---@field highlights table[]
---@field line_to_comment table<integer, kenjutu.PortedComment>
---@field comment_fold_ranges { fold_start_line: integer, resolved: boolean }[]

---@param comments kenjutu.PortedComment[]
---@param width integer
---@return kenjutu.BuildCommentListResult
function M.build_comment_list(comments, width)
  local lines = {}
  local fold_levels = {}
  local highlights = {}
  local line_to_comment = {}
  ---@type { fold_start_line: integer, resolved: boolean }[]
  local comment_fold_ranges = {}

  for i, pc in ipairs(comments) do
    if i > 1 then
      table.insert(lines, "")
      fold_levels[#lines] = "0"
    end

    local c = pc.comment
    local line_num = pc.ported_line and ("L" .. pc.ported_line) or "L?"
    local side = c.side or "?"
    local resolved_tag = c.resolved and " [resolved]" or ""
    local date = format_date(c.created_at)
    local header_prefix = line_num .. " (" .. side .. ")" .. resolved_tag
    local header = header_prefix .. string.rep(" ", math.max(1, width - #header_prefix - #date)) .. date

    table.insert(lines, header)
    fold_levels[#lines] = "0"
    line_to_comment[#lines] = pc
    if c.resolved then
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentResolved" })
    else
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentHeader" })
    end
    table.insert(
      highlights,
      { line = #lines - 1, col = #header - #date, end_col = #header, hl = "KenjutuCommentTimestamp" }
    )

    local body_lines = vim.split(c.body, "\n", { plain = true })
    for j, body_line in ipairs(body_lines) do
      table.insert(lines, "  " .. body_line)
      fold_levels[#lines] = j == 1 and ">1" or "1"
      line_to_comment[#lines] = pc
    end
    table.insert(comment_fold_ranges, { fold_start_line = #lines - #body_lines + 1, resolved = c.resolved })

    local target = c.anchor and c.anchor.target or {}
    for j, code_line in ipairs(target) do
      local rendered = "  \xe2\x94\x82 " .. code_line
      table.insert(lines, rendered)
      fold_levels[#lines] = j == 1 and ">2" or "2"
      line_to_comment[#lines] = pc
      table.insert(highlights, { line = #lines - 1, col = 2, end_col = 5, hl = "KenjutuCommentCodeGutter" })
      table.insert(highlights, { line = #lines - 1, col = 5, end_col = #rendered, hl = "KenjutuCommentCodeSnippet" })
    end

    local reply_list = c.replies or {}
    for _, reply in ipairs(reply_list) do
      local reply_sep = "  "
        .. string.rep("\xe2\x94\x80", math.max(1, width - 12))
        .. " Reply "
        .. string.rep("\xe2\x94\x80", 3)
      table.insert(lines, reply_sep)
      fold_levels[#lines] = "1"
      line_to_comment[#lines] = pc
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentReplyHeader" })

      for _, body_line in ipairs(vim.split(reply.body, "\n", { plain = true })) do
        table.insert(lines, "    " .. body_line)
        fold_levels[#lines] = "1"
        line_to_comment[#lines] = pc
      end

      local reply_date = format_date(reply.created_at)
      table.insert(lines, string.rep(" ", math.max(0, width - #reply_date)) .. reply_date)
      fold_levels[#lines] = "1"
      line_to_comment[#lines] = pc
      table.insert(highlights, { line = #lines - 1, hl = "KenjutuCommentTimestamp" })
    end
  end

  return {
    lines = lines,
    fold_levels = fold_levels,
    highlights = highlights,
    line_to_comment = line_to_comment,
    comment_fold_ranges = comment_fold_ranges,
  }
end

---@param result kenjutu.BuildCommentListResult
---@param buf integer
---@param win integer
local function render_comment_list(result, buf, win)
  vim.bo[buf].modifiable = true
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, result.lines)
  vim.bo[buf].modifiable = false

  vim.api.nvim_buf_clear_namespace(buf, NS, 0, -1)
  for _, hl in ipairs(result.highlights) do
    local col = hl.col or 0
    local end_col = hl.end_col or #result.lines[hl.line + 1]
    vim.api.nvim_buf_set_extmark(buf, NS, hl.line, col, {
      end_col = end_col,
      hl_group = hl.hl,
    })
  end

  vim.b[buf].kenjutu_fold_levels = result.fold_levels

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

---@class kenjutu.OpenCommentListOpts
---@field file_path string
---@field comments kenjutu.PortedComment[]
---@field on_select fun(pc: kenjutu.PortedComment)
---@field dir string
---@field change_id string
---@field on_resolve fun()

---@param opts kenjutu.OpenCommentListOpts
---@return integer|nil bufnr
---@return integer|nil winnr
function M.open_comment_list(opts)
  local comments = {}
  for _, pc in ipairs(opts.comments) do
    table.insert(comments, pc)
  end
  table.sort(comments, function(a, b)
    local la = a.ported_line or math.huge
    local lb = b.ported_line or math.huge
    return la < lb
  end)

  if #comments == 0 then
    vim.notify("No comments on this file", vim.log.levels.INFO)
    return nil, nil
  end

  local width = math.floor(vim.o.columns * 0.6)
  local result = M.build_comment_list(comments, width)

  local buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].swapfile = false
  vim.bo[buf].buflisted = false
  vim.bo[buf].modifiable = true
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, result.lines)
  vim.bo[buf].modifiable = false

  for _, hl in ipairs(result.highlights) do
    local col = hl.col or 0
    local end_col = hl.end_col or #result.lines[hl.line + 1]
    vim.api.nvim_buf_set_extmark(buf, NS, hl.line, col, {
      end_col = end_col,
      hl_group = hl.hl,
    })
  end

  vim.b[buf].kenjutu_fold_levels = result.fold_levels

  local height = math.min(#result.lines, math.floor(vim.o.lines * 0.7))
  local title = "Comments: " .. opts.file_path

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    row = math.floor((vim.o.lines - height) / 2),
    col = math.floor((vim.o.columns - width) / 2),
    width = width,
    height = height,
    style = "minimal",
    border = "rounded",
    title = " " .. title .. " ",
    title_pos = "center",
  })

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

  local closed = false
  local function close_float()
    if closed then
      return
    end
    closed = true
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
    if vim.api.nvim_buf_is_valid(buf) then
      vim.api.nvim_buf_delete(buf, { force = true })
    end
  end

  local function selected_comment()
    local cursor_line = vim.api.nvim_win_get_cursor(win)[1]
    return result.line_to_comment[cursor_line]
  end

  vim.keymap.set("n", "q", close_float, { buffer = buf, silent = true, nowait = true })
  vim.keymap.set("n", "<Esc>", close_float, { buffer = buf, silent = true, nowait = true })

  vim.keymap.set("n", "<CR>", function()
    local pc = selected_comment()
    if pc then
      close_float()
      opts.on_select(pc)
    end
  end, { buffer = buf, silent = true, nowait = true })

  vim.keymap.set("n", "x", function()
    local pc = selected_comment()
    if not pc then
      return
    end
    local resolve_fn = pc.comment.resolved and kjn.unresolve_comment or kjn.resolve_comment
    resolve_fn({
      dir = opts.dir,
      change_id = opts.change_id,
      file_path = opts.file_path,
      comment_id = pc.comment.id,
    }, function(err, _)
      if err then
        vim.notify("resolve comment: " .. err, vim.log.levels.ERROR)
        return
      end
      pc.comment.resolved = not pc.comment.resolved
      local cursor_line = vim.api.nvim_win_get_cursor(win)[1]
      result = M.build_comment_list(comments, width)
      render_comment_list(result, buf, win)
      local new_cursor = math.min(cursor_line, #result.lines)
      vim.api.nvim_win_set_cursor(win, { new_cursor, 0 })
      if opts.on_resolve then
        opts.on_resolve()
      end
    end)
  end, { buffer = buf, silent = true, nowait = true })

  vim.api.nvim_create_autocmd("WinClosed", {
    buffer = buf,
    once = true,
    callback = function()
      closed = true
    end,
  })

  return buf, win
end

---@param bufnr integer
---@param file_comments kenjutu.PortedComment[]
---@param side_filter "Old"|"New"|nil
function M.place_signs(bufnr, file_comments, side_filter)
  vim.fn.sign_unplace("kenjutu_comments", { buffer = bufnr })
  for _, pc in ipairs(file_comments) do
    if pc.ported_line and (not side_filter or pc.comment.side == side_filter) then
      local sign_name = pc.comment.resolved and "KenjutuCommentResolved" or "KenjutuComment"
      vim.fn.sign_place(0, "kenjutu_comments", sign_name, bufnr, { lnum = pc.ported_line })
    end
  end
end

function M.goto_next_comment()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local signs = comment_signs(bufnr)
  for _, line in ipairs(signs) do
    if line > cursor_line then
      vim.api.nvim_win_set_cursor(0, { line, 0 })
      return
    end
  end
end

function M.goto_prev_comment()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor_line = vim.api.nvim_win_get_cursor(0)[1]
  local signs = comment_signs(bufnr)
  for i = #signs, 1, -1 do
    if signs[i] < cursor_line then
      vim.api.nvim_win_set_cursor(0, { signs[i], 0 })
      return
    end
  end
end

return M
