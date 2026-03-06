vim.opt.rtp:prepend(vim.fn.getcwd())
require("tests.test").run()
