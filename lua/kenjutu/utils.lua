local M = {}

---@param file kenjutu.FileEntry
---@return string
function M.file_path(file)
  local p = file.newPath or file.oldPath
  assert(p ~= nil, "File entry must have either newPath or oldPath")
  return p
end

return M
