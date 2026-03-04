local M = {}

--- Run a kjn subcommand asynchronously and return parsed JSON.
---@param dir string working directory
---@param args string[] subcommand + flags (e.g., {"files", "--commit", sha})
---@param callback fun(err: string|nil, result: table|nil)
function M.run(dir, args, callback)
  local cmd = { "kjn", "--dir", dir }
  for _, arg in ipairs(args) do
    table.insert(cmd, arg)
  end

  vim.system(
    cmd,
    { text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        local err = "kjn failed"
        local stderr = obj.stderr or ""
        -- Try to parse structured error from stderr
        local ok, parsed = pcall(vim.fn.json_decode, vim.trim(stderr))
        if ok and type(parsed) == "table" and parsed.error then
          err = parsed.error
        elseif stderr ~= "" then
          err = vim.trim(stderr)
        end
        callback(err, nil)
        return
      end

      local stdout = obj.stdout or ""
      if stdout == "" then
        callback(nil, nil)
        return
      end

      local ok, result = pcall(vim.fn.json_decode, stdout)
      if not ok then
        callback("failed to parse kjn output: " .. tostring(result), nil)
        return
      end

      callback(nil, result)
    end)
  )
end

return M
