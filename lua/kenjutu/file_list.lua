local file_render = require("kenjutu.file_render")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_file_list")

--- Render a file tree into the buffer.
--- Returns a line_map that maps 1-indexed buffer line numbers to file entries.
--- Lines without a file (header, blank, directory) are absent from the map.
---@param bufnr integer
---@param files kenjutu.FileEntry[]
---@param winnr integer
---@return table<integer, kenjutu.FileEntry> line_map
function M.render(bufnr, files, winnr)
  local render_lines = {} ---@type kenjutu.RenderLine[]

  local reviewed = file_render.count_reviewed(files)
  local header = string.format(" Files %d/%d", reviewed, #files)
  table.insert(render_lines, { text = header, highlights = { { 0, #header, "KenjutuHeader" } } })
  table.insert(render_lines, { text = "", highlights = {} })

  local tree = file_render.build_tree(files)
  local tree_lines, line_map = file_render.flatten_tree(tree, #render_lines + 1)
  vim.list_extend(render_lines, tree_lines)

  file_render.apply_to_buffer(bufnr, render_lines, ns)
  return line_map
end

return M
