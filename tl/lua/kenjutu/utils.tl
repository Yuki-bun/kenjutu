local record FileEntry
   oldPath: string
   newPath: string
   status: string
   additions: integer
   deletions: integer
   isBinary: boolean
   reviewStatus: string
end

local record M
   FileEntry: type = FileEntry
   file_path: function(FileEntry): string
   await_all: function({string: function(function(any, any))}, function(any, {string: any}))
end

function M.file_path(file: FileEntry): string
   local p = file.newPath or file.oldPath
   assert(p ~= nil, "File entry must have either newPath or oldPath")
   return p as string
end

function M.await_all(tasks: {string: function(function(any, any))}, callback: function(any, {string: any}))
   local results: {string: any} = {}
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
      func(function(err: any, result: any)
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
