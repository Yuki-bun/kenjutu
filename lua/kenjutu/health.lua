local M = {}

local function check_kjn()
  local bin = require("kenjutu.install").bin_path()
  if bin == nil then
    vim.health.error("kjn binary is not installed")
    return
  end

  local ok, obj = pcall(vim.system, { bin, "--version" })
  if not ok then
    vim.health.error("kjn binary is broken")
    return
  end
  local result = obj:wait()
  local bin_version = result.stdout
  assert(bin_version, "kjn should output version string")
  local plug_version = require("kenjutu.version")
  if not vim.version.eq(bin_version, plug_version) then
    vim.health.error(string.format("kjn binary version mismatch. Requires v%s found %s", plug_version, bin_version))
    return
  end

  vim.health.ok("kjn installed")
end

local function check_telescope()
  local ok = pcall(require, "telescope")
  if ok then
    vim.health.ok("telescope installed (optional)")
  else
    vim.health.warn("telescope is not installed. Optional but required for some features")
  end
end

M.check = function()
  vim.health.start("kenjutu")

  check_kjn()
  check_telescope()
end

return M
