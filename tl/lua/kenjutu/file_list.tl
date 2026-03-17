local file_render = require("kenjutu.file_render")

local record M
end

local ns = vim.api.nvim_create_namespace("kenjutu_file_list")

function M.render(bufnr: integer, files: {file_render.FileEntry}, winnr: integer): {integer: file_render.FileEntry}
   local render_lines: {file_render.RenderLine} = {}

   local reviewed = file_render.count_reviewed(files)
   local header = string.format(" Files %d/%d", reviewed, #files)
   table.insert(render_lines, { text = header, highlights = { { 0, #header, "KenjutuHeader" } } } as file_render.RenderLine)
   table.insert(render_lines, { text = "", highlights = {} } as file_render.RenderLine)

   local tree = file_render.build_tree(files)
   local tree_lines, line_map = file_render.flatten_tree(tree, #render_lines + 1)
   vim.list_extend(render_lines, tree_lines)

   file_render.apply_to_buffer(bufnr, render_lines, ns)
   return line_map
end

return M
