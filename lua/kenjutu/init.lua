local M = {}

function M.setup() end

function M.log()
  require("kenjutu.log").open()
end

function M.pr()
  require("kenjutu.pr_picker").open(vim.fn.getcwd())
end

return M
