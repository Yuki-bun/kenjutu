local deps_path = vim.fn.stdpath("data") .. "/kenjutu-test-deps"

local mini_path = deps_path .. "/mini.nvim"
if not vim.loop.fs_stat(mini_path) then
  vim.fn.system({
    "git",
    "clone",
    "--depth",
    "1",
    "https://github.com/echasnovski/mini.nvim",
    mini_path,
  })
end
vim.opt.rtp:prepend(mini_path)

vim.opt.rtp:prepend(vim.fn.getcwd())

require("mini.test").setup()
