local utils = require("kenjutu.utils")

local record FileEntry
   oldPath: string
   newPath: string
   status: string
   additions: integer
   deletions: integer
   isBinary: boolean
   reviewStatus: string
end

local record RenderLine
   text: string
   highlights: {{any}}
end

local record TreeNode
   type: string
   name: string
   path: string
   file: FileEntry
   children: {TreeNode}
end

local record HlDef
   fg: string
   bold: boolean
end

local record M
   type FileEntry = FileEntry
   type RenderLine = RenderLine
   type TreeNode = TreeNode

   review_indicator: function(string): string, string
   status_indicator: function(string): string, string
   count_reviewed: function({FileEntry}): integer
   build_tree: function({FileEntry}): {TreeNode}
   format_file_line: function(FileEntry, string): RenderLine
   format_dir_line: function(string, string): RenderLine
   flatten_tree: function({TreeNode}, integer): {RenderLine}, {integer: FileEntry}
   apply_to_buffer: function(integer, {RenderLine}, integer)

   _test: {string: any}
end

local hl_defs: {string: HlDef} = {
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
   KenjutuCommitSummary = { fg = "#cdd6f4", bold = true },
   KenjutuCommitDescription = { fg = "#a6adc8" },
   KenjutuCommitAuthor = { fg = "#94e2d5" },
   KenjutuCommitTimestamp = { fg = "#6c7086" },
}

for name, def in pairs(hl_defs) do
   vim.api.nvim_set_hl(0, name, def as {string: any})
end

function M.review_indicator(status: string): string, string
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

function M.status_indicator(status: string): string, string
   local map: {string: {string}} = {
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

function M.count_reviewed(files: {FileEntry}): integer
   local n = 0
   for _, f in ipairs(files) do
      if f.reviewStatus == "reviewed" then
         n = n + 1
      end
   end
   return n
end

local function insert_into_tree(parent: TreeNode, parts: {string}, file: FileEntry)
   if #parts == 1 then
      local node: TreeNode = {
         type = "file",
         name = parts[1],
         path = utils.file_path(file as utils.FileEntry),
         file = file,
      }
      table.insert(parent.children, node)
      return
   end

   local dir_name = parts[1]
   local rest: {string} = { unpack(parts, 2) }

   for _, child in ipairs(parent.children) do
      if child.type == "directory" and child.name == dir_name then
         insert_into_tree(child, rest, file)
         return
      end
   end

   local new_dir: TreeNode = {
      type = "directory",
      name = dir_name,
      path = parent.path ~= "" and (parent.path .. "/" .. dir_name) or dir_name,
      children = {},
   }
   table.insert(parent.children, new_dir)
   insert_into_tree(new_dir, rest, file)
end

local function sort_tree(nodes: {TreeNode}): {TreeNode}
   local sorted: {TreeNode} = { unpack(nodes) }
   table.sort(sorted, function(a: TreeNode, b: TreeNode): boolean
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

local function compact_tree(nodes: {TreeNode}): {TreeNode}
   local result: {TreeNode} = {}
   for _, node in ipairs(nodes) do
      if node.type == "file" then
         table.insert(result, node)
      else
         local name = node.name
         local current = node
         local single_child: TreeNode = #current.children == 1 and current.children[1] or nil
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

function M.build_tree(files: {FileEntry}): {TreeNode}
   local root: TreeNode = { type = "directory", name = "", path = "", children = {} }

   for _, file in ipairs(files) do
      local path = utils.file_path(file as utils.FileEntry)
      local parts: {string} = vim.split(path, "/")
      insert_into_tree(root, parts, file)
   end

   return compact_tree(sort_tree(root.children))
end

function M.format_file_line(file: FileEntry, indent: string): RenderLine
   local indicator, indicator_hl = M.review_indicator(file.reviewStatus)
   local path_name: string = file.newPath and vim.fn.fnamemodify(file.newPath, ":t") or vim.fn.fnamemodify(file.oldPath, ":t")
   local status_char, status_hl = M.status_indicator(file.status)

   local parts: {string} = {}
   local highlights: {{any}} = {}
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
         stats = stats .. " +" .. tostring(file.additions)
      end
      if file.deletions > 0 then
         stats = stats .. " -" .. tostring(file.deletions)
      end
      table.insert(parts, stats)
      table.insert(highlights, { col, col + #stats, "KenjutuStats" })
   end

   return { text = table.concat(parts), highlights = highlights }
end

function M.format_dir_line(name: string, prefix: string): RenderLine
   local text = prefix .. name
   return {
      text = text,
      highlights = { { #prefix, #text, "KenjutuDir" } },
   }
end

local function tree_prefix(ancestors: {boolean}, is_last: boolean): string
   local parts: {string} = {}
   for _, continues in ipairs(ancestors) do
      table.insert(parts, continues and "│ " or "  ")
   end
   if #ancestors >= 0 and is_last ~= nil then
      table.insert(parts, is_last and "└ " or "├ ")
   end
   return table.concat(parts)
end

local function flatten_nodes(nodes: {TreeNode}, out: {RenderLine}, line_map: {integer: FileEntry}, offset: integer, ancestors: {boolean})
   for i, node in ipairs(nodes) do
      local is_last = i == #nodes
      local prefix = tree_prefix(ancestors, is_last)
      if node.type == "directory" then
         table.insert(out, M.format_dir_line(node.name, prefix))
         local child_ancestors: {boolean} = { unpack(ancestors) }
         table.insert(child_ancestors, not is_last)
         flatten_nodes(node.children, out, line_map, offset, child_ancestors)
      else
         table.insert(out, M.format_file_line(node.file, prefix))
         line_map[offset + #out] = node.file
      end
   end
end

function M.flatten_tree(nodes: {TreeNode}, start_line: integer): {RenderLine}, {integer: FileEntry}
   local out: {RenderLine} = {}
   local line_map: {integer: FileEntry} = {}
   flatten_nodes(nodes, out, line_map, start_line - 1, {})
   return out, line_map
end

function M.apply_to_buffer(bufnr: integer, render_lines: {RenderLine}, ns: integer)
   local lines: {string} = {}
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
         } as {string: any})
      end
   end
end

M._test = {
   build_tree = M.build_tree as any,
   review_indicator = M.review_indicator as any,
   status_indicator = M.status_indicator as any,
   format_file_line = M.format_file_line as any,
   format_dir_line = M.format_dir_line as any,
   count_reviewed = M.count_reviewed as any,
}

return M
