local gh = require("kenjutu.gh")
local pr_mod = require("kenjutu.pr")

local M = {}

---@param decision string
---@return string
local function review_badge(decision)
  if decision == "APPROVED" then
    return "✓ approved"
  elseif decision == "CHANGES_REQUESTED" then
    return "✗ changes requested"
  elseif decision == "REVIEW_REQUIRED" then
    return "● review required"
  end
  return decision
end

---@param dir string
function M.open(dir)
  local has_telescope, _ = pcall(require, "telescope")
  if not has_telescope then
    vim.notify("telescope.nvim is required for PR picker", vim.log.levels.ERROR)
    return
  end

  local pickers = require("telescope.pickers")
  local finders = require("telescope.finders")
  local conf = require("telescope.config").values
  local previewers = require("telescope.previewers")
  local actions = require("telescope.actions")
  local action_state = require("telescope.actions.state")
  local entry_display = require("telescope.pickers.entry_display")

  gh.list_prs(dir, function(err, prs)
    if err then
      vim.notify("gh pr list: " .. err, vim.log.levels.ERROR)
      return
    end
    if not prs or #prs == 0 then
      vim.notify("No open pull requests", vim.log.levels.INFO)
      return
    end

    local displayer = entry_display.create({
      separator = " ",
      items = {
        { width = 6 },
        { width = 60 },
        { remaining = true },
      },
    })

    local function make_display(entry)
      local pr = entry.value
      local draft_marker = pr.isDraft and " (draft)" or ""
      return displayer({
        { "#" .. tostring(pr.number), "Identifier" },
        { pr.title .. draft_marker },
        { pr.author.login, "Comment" },
      })
    end

    pickers
      .new({}, {
        prompt_title = "Pull Requests",
        finder = finders.new_table({
          results = prs,
          entry_maker = function(pr)
            return {
              value = pr,
              display = make_display,
              ordinal = tostring(pr.number) .. " " .. pr.title .. " " .. pr.author.login,
            }
          end,
        }),
        sorter = conf.generic_sorter({}),
        previewer = previewers.new_buffer_previewer({
          title = "Pull Request",
          define_preview = function(self, entry)
            local pr = entry.value
            local width = vim.api.nvim_win_get_width(self.state.winid)
            local preview_lines = {}
            local preview_hls = {}

            local title = string.format("#%d  %s", pr.number, pr.title)
            table.insert(preview_lines, title)
            table.insert(preview_hls, { line = 0, hl = "Title" })
            table.insert(preview_lines, "")

            table.insert(preview_lines, string.format("Branch: %s → %s", pr.headRefName, pr.baseRefName))
            table.insert(preview_lines, string.format("Author: %s", pr.author.name or pr.author.login))
            table.insert(
              preview_lines,
              string.format("Files:  %d  (+%d / -%d)", pr.changedFiles, pr.additions, pr.deletions)
            )
            table.insert(preview_lines, string.format("Review: %s", review_badge(pr.reviewDecision)))

            if pr.isDraft then
              table.insert(preview_lines, "Status: Draft")
            end

            table.insert(preview_lines, "")
            table.insert(preview_lines, string.rep("─", width))
            table.insert(preview_lines, "")

            if pr.body and pr.body ~= "" then
              for _, body_line in ipairs(vim.split(pr.body, "\n", { plain = true })) do
                table.insert(preview_lines, body_line)
              end
            end

            vim.api.nvim_buf_set_lines(self.state.bufnr, 0, -1, false, preview_lines)

            local preview_ns = vim.api.nvim_create_namespace("kenjutu_pr_preview")
            for _, hl in ipairs(preview_hls) do
              pcall(vim.api.nvim_buf_set_extmark, self.state.bufnr, preview_ns, hl.line, 0, {
                end_col = #preview_lines[hl.line + 1],
                hl_group = hl.hl,
              })
            end
          end,
        }),
        attach_mappings = function(prompt_bufnr)
          actions.select_default:replace(function()
            actions.close(prompt_bufnr)
            local selection = action_state.get_selected_entry()
            if not selection then
              return
            end
            pr_mod.PrScreenState.new(dir, selection.value)
          end)
          return true
        end,
      })
      :find()
  end)
end

return M
