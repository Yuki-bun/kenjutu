local kjn = require("kenjutu.kjn")

local M = {}

local PREVIEW_NS = vim.api.nvim_create_namespace("kenjutu_comment_picker_preview")

---@param bufnr integer
---@param winnr integer
---@param comment kenjutu.MaterializedComment
---@param file_path string
local function render_preview(bufnr, winnr, comment, file_path)
  local width = vim.api.nvim_win_get_width(winnr)
  local lines = {}
  local highlights = {}
  local code_line_count = 0

  table.insert(lines, file_path)
  table.insert(highlights, { line = 0, hl = "Title" })
  table.insert(lines, "")

  local target_start, target_end
  local anchor = comment.anchor
  if anchor and (anchor.before or anchor.target or anchor.after) then
    for _, l in ipairs(anchor.before or {}) do
      table.insert(lines, l)
    end
    target_start = #lines
    for _, l in ipairs(anchor.target or {}) do
      table.insert(lines, l)
    end
    target_end = #lines
    for _, l in ipairs(anchor.after or {}) do
      table.insert(lines, l)
    end
    code_line_count = #lines

    table.insert(lines, "")
    table.insert(lines, string.rep("─", width))
    table.insert(highlights, { line = #lines - 1, hl = "Comment" })
    table.insert(lines, "")
  end

  for _, body_line in ipairs(vim.split(comment.body, "\n", { plain = true })) do
    table.insert(lines, body_line)
  end

  for _, reply in ipairs(comment.replies or {}) do
    table.insert(lines, "")
    table.insert(lines, string.rep("─", width))
    table.insert(highlights, { line = #lines - 1, hl = "Comment" })
    table.insert(lines, "")
    for _, body_line in ipairs(vim.split(reply.body, "\n", { plain = true })) do
      table.insert(lines, "  " .. body_line)
    end
  end

  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)

  if code_line_count > 0 then
    local ft = vim.filetype.match({ filename = file_path })
    if ft then
      local lang = vim.treesitter.language.get_lang(ft) or ft
      local ok = pcall(vim.treesitter.start, bufnr, lang)
      if ok then
        vim.api.nvim_buf_set_extmark(bufnr, PREVIEW_NS, code_line_count, 0, {
          end_row = #lines,
          hl_group = "Normal",
          priority = 200,
        })
      end
    end
  end

  if target_start and target_end and target_start < target_end then
    for i = target_start, target_end - 1 do
      vim.api.nvim_buf_set_extmark(bufnr, PREVIEW_NS, i, 0, {
        sign_text = "│",
        sign_hl_group = "DiagnosticInfo",
        priority = 300,
      })
    end
  end

  for _, hl in ipairs(highlights) do
    local end_col = hl.end_col or #lines[hl.line + 1]
    vim.api.nvim_buf_set_extmark(bufnr, PREVIEW_NS, hl.line, hl.col or 0, {
      end_col = end_col,
      hl_group = hl.hl,
      priority = 300,
    })
  end
end

---@class kenjutu.CommentPickerOpts
---@field dir string
---@field commit_id string
---@field on_select fun(file_path: string, pc: kenjutu.PortedComment)

---@param opts kenjutu.CommentPickerOpts
function M.open(opts)
  local has_telescope, _ = pcall(require, "telescope")
  if not has_telescope then
    vim.notify("telescope.nvim is required for comment picker", vim.log.levels.ERROR)
    return
  end

  local pickers = require("telescope.pickers")
  local finders = require("telescope.finders")
  local conf = require("telescope.config").values
  local previewers = require("telescope.previewers")
  local actions = require("telescope.actions")
  local action_state = require("telescope.actions.state")
  local entry_display = require("telescope.pickers.entry_display")

  kjn.get_comments(opts.dir, opts.commit_id, function(err, result)
    if err then
      vim.notify("Error loading comments: " .. err, vim.log.levels.ERROR)
      return
    end

    ---@type { file_path: string, pc: kenjutu.PortedComment }[]
    local entries = {}
    for _, file_comments in ipairs(result and result.files or {}) do
      for _, pc in ipairs(file_comments.comments) do
        table.insert(entries, { file_path = file_comments.file_path, pc = pc })
      end
    end

    if #entries == 0 then
      vim.notify("No comments on this change", vim.log.levels.INFO)
      return
    end

    local displayer = entry_display.create({
      separator = " ",
      items = {
        { width = 1 },
        { remaining = true },
      },
    })

    local function make_display(entry)
      local pc = entry.value.pc
      local c = pc.comment
      local line_str = pc.ported_line and tostring(pc.ported_line) or "?"
      local first_line = vim.split(c.body, "\n", { plain = true })[1]
      local location = entry.value.file_path .. ":" .. line_str .. " (" .. c.side .. ")"
      local resolved = c.resolved

      return displayer({
        { resolved and "✓" or "●", resolved and "DiagnosticOk" or "DiagnosticError" },
        { location .. "  " .. first_line, resolved and "Comment" or nil },
      })
    end

    local function make_finder()
      return finders.new_table({
        results = entries,
        entry_maker = function(item)
          local c = item.pc.comment
          local line_str = item.pc.ported_line and tostring(item.pc.ported_line) or "?"
          local first_line = vim.split(c.body, "\n", { plain = true })[1]
          local ordinal = item.file_path .. ":" .. line_str .. " " .. c.side .. " " .. first_line

          return {
            value = item,
            display = make_display,
            ordinal = ordinal,
            text = ordinal,
          }
        end,
      })
    end

    local function toggle_resolved(prompt_bufnr)
      local entry = action_state.get_selected_entry()
      if not entry then
        return
      end
      local item = entry.value
      local c = item.pc.comment
      local resolve_opts = {
        dir = opts.dir,
        commit_id = opts.commit_id,
        file_path = item.file_path,
        comment_id = c.id,
      }
      local fn = c.resolved and kjn.unresolve_comment or kjn.resolve_comment
      fn(resolve_opts, function(err, _)
        if err then
          vim.notify("Error toggling resolved: " .. err, vim.log.levels.ERROR)
          return
        end
        c.resolved = not c.resolved
        local picker = action_state.get_current_picker(prompt_bufnr)
        picker:refresh(make_finder(), { reset_prompt = false })
      end)
    end

    pickers
      .new({}, {
        prompt_title = "Comments",
        finder = make_finder(),
        sorter = conf.generic_sorter({}),
        previewer = previewers.new_buffer_previewer({
          title = "Comment Thread",
          define_preview = function(self, entry, _status)
            render_preview(self.state.bufnr, self.state.winid, entry.value.pc.comment, entry.value.file_path)
          end,
        }),
        attach_mappings = function(prompt_bufnr, map)
          actions.select_default:replace(function()
            actions.close(prompt_bufnr)
            local selection = action_state.get_selected_entry()
            if selection then
              opts.on_select(selection.value.file_path, selection.value.pc)
            end
          end)
          map({ "i", "n" }, "<C-x>", function()
            toggle_resolved(prompt_bufnr)
          end, { desc = "toggle resolved" })
          return true
        end,
      })
      :find()
  end)
end

return M
