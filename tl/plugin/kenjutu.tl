local record CmdOpts
   fargs: {string}
end

vim.api.nvim_create_user_command("Kenjutu", function(opts_raw: any)
  local opts = opts_raw as CmdOpts
  local subcmd = opts.fargs[1]
  if subcmd == "log" then
    require("kenjutu").log()
  else
    vim.notify("Unknown subcommand: " .. (subcmd or ""), vim.log.levels.ERROR)
  end
end, {
  nargs = "+",
  complete = function(): {string}
    return { "log" }
  end,
} as {string:any})
