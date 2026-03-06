local kjn = require("kenjutu.kjn")
local utils = require("kenjutu.utils")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_file_tree")

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
  KenjutuDir = { fg = "#89b4fa", bold = true },
}

for name, def in pairs(hl_defs) do
  vim.api.nvim_set_hl(0, name, def)
end

---@class kenjutu.FileNode
---@field type "file"
---@field name string
---@field path string
---@field file kenjutu.FileEntry

---@class kenjutu.DirNode
---@field type "directory"
---@field name string
---@field path string
---@field children kenjutu.TreeNode[]

---@alias kenjutu.TreeNode kenjutu.FileNode | kenjutu.DirNode

---@param parent kenjutu.DirNode
---@param parts string[]
---@param file kenjutu.FileEntry
local function insert_into_tree(parent, parts, file)
  if #parts == 1 then
    ---@type kenjutu.FileNode
    local node = {
      type = "file",
      name = parts[1],
      path = utils.file_path(file),
      file = file,
    }
    table.insert(parent.children, node)
    return
  end

  local dir_name = parts[1]
  local rest = { unpack(parts, 2) }

  for _, child in ipairs(parent.children) do
    if child.type == "directory" and child.name == dir_name then
      insert_into_tree(child, rest, file)
      return
    end
  end

  ---@type kenjutu.DirNode
  local new_dir = {
    type = "directory",
    name = dir_name,
    path = parent.path ~= "" and (parent.path .. "/" .. dir_name) or dir_name,
    children = {},
  }
  table.insert(parent.children, new_dir)
  insert_into_tree(new_dir, rest, file)
end

---@param nodes kenjutu.TreeNode[]
---@return kenjutu.TreeNode[]
local function sort_tree(nodes)
  local sorted = { unpack(nodes) }
  table.sort(sorted, function(a, b)
    if a.type == "directory" and b.type == "file" then
      return true
    end
    if a.type == "file" and b.type == "directory" then
      return false
    end
    return a.name < b.name
  end)

  for i, node in ipairs(sorted) do
    if node.type == "directory" then
      sorted[i] = {
        type = node.type,
        name = node.name,
        path = node.path,
        children = sort_tree(node.children),
      }
    end
  end

  return sorted
end

---@param nodes kenjutu.TreeNode[]
---@return kenjutu.TreeNode[]
local function compact_tree(nodes)
  local result = {}
  for _, node in ipairs(nodes) do
    if node.type == "file" then
      table.insert(result, node)
    else
      local name = node.name
      local current = node
      local single_child = #current.children == 1 and current.children[1] or nil
      while single_child and single_child.type == "directory" do
        name = name .. "/" .. single_child.name
        current = single_child
        single_child = #current.children == 1 and current.children[1] or nil
      end
      table.insert(result, {
        type = current.type,
        name = name,
        path = current.path,
        children = compact_tree(current.children),
      })
    end
  end
  return result
end

---@param files kenjutu.FileEntry[]
---@return kenjutu.TreeNode[]
local function build_tree(files)
  ---@type kenjutu.DirNode
  local root = { type = "directory", name = "", path = "", children = {} }

  for _, file in ipairs(files) do
    local path = utils.file_path(file)
    local parts = vim.split(path, "/")
    insert_into_tree(root, parts, file)
  end

  return compact_tree(sort_tree(root.children))
end

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

---@param files kenjutu.FileEntry[]
---@return integer
local function count_reviewed(files)
  local n = 0
  for _, f in ipairs(files) do
    if f.reviewStatus == "reviewed" then
      n = n + 1
    end
  end
  return n
end

---@class kenjutu.RenderLine
---@field text string
---@field highlights {[1]: integer, [2]: integer, [3]: string}[]

---@param file kenjutu.FileEntry
---@param indent string
---@return kenjutu.RenderLine
local function format_file_line(file, indent)
  local indicator, indicator_hl = review_indicator(file.reviewStatus)
  local path_name = file.newPath and vim.fn.fnamemodify(file.newPath, ":t") or vim.fn.fnamemodify(file.oldPath, ":t")
  local status_char, status_hl = status_indicator(file.status)

  local parts = {}
  local highlights = {}
  local col = 0

  table.insert(parts, indent)
  col = col + #indent

  table.insert(parts, indicator)
  if indicator_hl then
    table.insert(highlights, { col, col + #indicator, indicator_hl })
  end
  col = col + #indicator

  table.insert(parts, "  ")
  col = col + 2

  table.insert(parts, path_name)
  col = col + #path_name

  local status_str = " " .. status_char
  table.insert(parts, status_str)
  table.insert(highlights, { col + 1, col + 1 + #status_char, status_hl })
  col = col + #status_str

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
  end

  return { text = table.concat(parts), highlights = highlights }
end

---@param name string
---@param indent string
---@return kenjutu.RenderLine
local function format_dir_line(name, indent)
  local prefix = indent .. "  "
  local text = prefix .. name
  return {
    text = text,
    highlights = { { #prefix, #text, "KenjutuDir" } },
  }
end

---@param nodes kenjutu.TreeNode[]
---@param depth integer
---@param out kenjutu.RenderLine[]
local function flatten_tree(nodes, depth, out)
  local indent = string.rep("  ", depth)
  for _, node in ipairs(nodes) do
    if node.type == "directory" then
      table.insert(out, format_dir_line(node.name, indent))
      flatten_tree(node.children, depth + 1, out)
    else
      table.insert(out, format_file_line(node.file, indent))
    end
  end
end

---@param bufnr integer
---@param files kenjutu.FileEntry[]
---@param winnr integer
local function render(bufnr, files, winnr)
  local render_lines = {} ---@type kenjutu.RenderLine[]

  local reviewed = count_reviewed(files)
  local header = string.format(" Files %d/%d", reviewed, #files)
  table.insert(render_lines, { text = header, highlights = { { 0, #header, "KenjutuHeader" } } })
  table.insert(render_lines, { text = "", highlights = {} })

  local tree = build_tree(files)
  flatten_tree(tree, 0, render_lines)

  local lines = {}
  for _, rl in ipairs(render_lines) do
    table.insert(lines, rl.text)
  end

  vim.bo[bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].modifiable = false

  vim.api.nvim_buf_clear_namespace(bufnr, ns, 0, -1)
  for i, rl in ipairs(render_lines) do
    for _, hl in ipairs(rl.highlights) do
      pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, i - 1, hl[1], {
        end_col = hl[2],
        hl_group = hl[3],
      })
    end
  end

  if vim.api.nvim_win_is_valid(winnr) then
    vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  end
end

---@class kenjutu.FileTreeState
---@field bufnr integer
---@field winnr integer
---@field dir string
---@field current_change_id string|nil
local FileTreeState = {}
FileTreeState.__index = FileTreeState

---@param dir string
---@param log_winnr integer
---@return kenjutu.FileTreeState
function FileTreeState.new(dir, log_winnr)
  local prev_win = vim.api.nvim_get_current_win()
  vim.api.nvim_set_current_win(log_winnr)

  vim.cmd("rightbelow vsplit")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_win_set_buf(winnr, bufnr)

  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-log-files"
  vim.bo[bufnr].modifiable = false

  vim.wo[winnr].cursorline = false
  vim.wo[winnr].number = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false
  vim.wo[winnr].winfixwidth = false

  vim.cmd("wincmd =")
  vim.api.nvim_set_current_win(prev_win)

  ---@type kenjutu.FileTreeState
  local state = {
    bufnr = bufnr,
    winnr = winnr,
    dir = dir,
    current_change_id = nil,
  }
  return setmetatable(state, FileTreeState)
end

---@param commit kenjutu.Commit
function FileTreeState:update(commit)
  if self.current_change_id == commit.change_id then
    return
  end
  self.current_change_id = commit.change_id

  local bufnr = self.bufnr
  local winnr = self.winnr

  kjn.run(self.dir, { "files", "--change-id", commit.change_id }, function(err, result)
    if err or not result then
      return
    end
    if not vim.api.nvim_buf_is_valid(bufnr) then
      return
    end
    if not vim.api.nvim_win_is_valid(winnr) then
      return
    end
    render(bufnr, result.files or {}, winnr)
  end)
end

function FileTreeState:close()
  if vim.api.nvim_win_is_valid(self.winnr) then
    vim.api.nvim_win_close(self.winnr, true)
  end
  if vim.api.nvim_buf_is_valid(self.bufnr) then
    vim.api.nvim_buf_delete(self.bufnr, { force = true })
  end
end

M.FileTreeState = FileTreeState

return M
