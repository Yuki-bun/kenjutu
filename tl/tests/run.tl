vim.opt.rtp:prepend(vim.fn.getcwd() as string)

local record TestModule
   run: function()
end

local test = require("tests.test") as TestModule
test.run()
