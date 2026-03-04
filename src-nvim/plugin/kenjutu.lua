vim.api.nvim_create_user_command("Kenjutu", function(opts)
  local subcmd = opts.fargs[1]
  if subcmd == "log" then
    require("kenjutu").log()
  else
    vim.notify("Unknown subcommand: " .. (subcmd or ""), vim.log.levels.ERROR)
  end
end, {
  nargs = "+",
  complete = function()
    return { "log" }
  end,
})
