local M = {}

---@param file kenjutu.FileEntry
---@return string
function M.file_path(file)
  local p = file.newPath or file.oldPath
  assert(p ~= nil, "File entry must have either newPath or oldPath")
  return p
end

--- Run async functions in parallel and collect their results.
--- Each task calls `cb(err, result)` when done. If any task errors,
--- the callback fires immediately with that error and remaining tasks
--- are ignored. On success, the callback receives a `{key = result}` map.
---@param tasks table<string, fun(cb: fun(err: any, result: any))>
---@param callback fun(err: any, results: table<string, any>|nil)
function M.await_all(tasks, callback)
  local results = {}
  local pending = 0
  local settled = false

  for _ in pairs(tasks) do
    pending = pending + 1
  end

  if pending == 0 then
    callback(nil, results)
    return
  end

  for key, func in pairs(tasks) do
    func(function(err, result)
      if settled then
        return
      end
      if err then
        settled = true
        callback(err, nil)
        return
      end
      results[key] = result
      pending = pending - 1
      if pending == 0 then
        settled = true
        callback(nil, results)
      end
    end)
  end
end

return M
