local kjn = require("kenjutu.kjn")

local M = {}

vim.api.nvim_set_hl(0, "KenjutuCommentSign", { default = true, fg = "#90ee90" })
vim.api.nvim_set_hl(0, "KenjutuCommentSignResolved", { default = true, fg = "#4a6e4a" })

vim.fn.sign_define("KenjutuComment", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSign" })
vim.fn.sign_define("KenjutuCommentResolved", { text = "\xe2\x96\x8e", texthl = "KenjutuCommentSignResolved" })

vim.api.nvim_set_hl(0, "KenjutuCommentTimestamp", { default = true, link = "Comment" })
vim.api.nvim_set_hl(0, "KenjutuCommentSeparator", { default = true, link = "Comment" })
vim.api.nvim_set_hl(0, "KenjutuCommentResolved", { default = true, link = "DiagnosticOk" })

local NS = vim.api.nvim_create_namespace("kenjutu_comment_thread")

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

  local width = math.min(60, math.floor(vim.o.columns * 0.6))
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

  local width = math.min(60, math.floor(vim.o.columns * 0.6))
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

---@param bufnr integer
---@param file_comments kenjutu.PortedComment[]
---@param side_filter "Old"|"New"|nil
function M.place_signs(bufnr, file_comments, side_filter)
  vim.fn.sign_unplace("kenjutu_comments", { buffer = bufnr })
  for _, pc in ipairs(file_comments) do
    if not side_filter or pc.comment.side == side_filter then
      local line = pc.ported_line or 1
      local sign_name = pc.comment.resolved and "KenjutuCommentResolved" or "KenjutuComment"
      vim.fn.sign_place(0, "kenjutu_comments", sign_name, bufnr, { lnum = line })
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
