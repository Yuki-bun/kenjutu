local record M
   setup: function()
   log: function()
end

function M.setup()
end

function M.log()
  local log = require("kenjutu.log") as {string: function()}
  log.open()
end

return M
