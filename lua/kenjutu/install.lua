local M = {}

local REPO = "Yuki-bun/kenjutu"
local plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")

---@return string
local function get_version()
  return require("kenjutu.version")
end

---@return string arch "x86_64" or "aarch64"
---@return string os_name "linux" or "darwin"
local function detect_platform()
  local uname = vim.uv.os_uname()

  local sysname = uname.sysname:lower()
  if sysname ~= "linux" and sysname ~= "darwin" then
    error("unsupported OS: " .. uname.sysname)
  end

  local machine = uname.machine
  if machine == "arm64" then
    machine = "aarch64"
  end
  if machine ~= "x86_64" and machine ~= "aarch64" then
    error("unsupported architecture: " .. uname.machine)
  end

  return machine, sysname
end

---@param artifact string
---@return string
local function release_url(artifact)
  local version = get_version()
  return string.format("https://github.com/%s/releases/download/kjn%%2Fv%s/%s", REPO, version, artifact)
end

---@param url string
---@param dest string
---@return boolean success
---@return string? error
local function download(url, dest)
  local result = vim.fn.system({ "curl", "-fSL", "--proto", "=https", "--retry", "3", "-o", dest, url })
  if vim.v.shell_error ~= 0 then
    return false, "download failed: " .. vim.fn.trim(result)
  end
  return true, nil
end

---@param file string
---@param expected_hash string
---@return boolean
local function verify_checksum(file, expected_hash)
  local result = vim.fn.trim(vim.fn.system({ "sha256sum", file }))
  if vim.v.shell_error ~= 0 then
    result = vim.fn.trim(vim.fn.system({ "shasum", "-a", "256", file }))
    if vim.v.shell_error ~= 0 then
      return false
    end
  end
  local actual_hash = result:match("^(%S+)")
  return actual_hash == expected_hash
end

---@param checksums_content string
---@param artifact string
---@return string?
local function parse_checksum(checksums_content, artifact)
  for line in checksums_content:gmatch("[^\n]+") do
    local hash, name = line:match("^(%S+)%s+(%S+)")
    if name == artifact then
      return hash
    end
  end
  return nil
end

function M.download()
  local arch, os_name = detect_platform()
  local artifact = string.format("kjn-%s-%s", arch, os_name)
  local version = get_version()

  local bin_dir = plugin_dir .. "/bin"
  vim.fn.mkdir(bin_dir, "p")

  local bin_path = bin_dir .. "/kjn"
  local checksums_path = bin_dir .. "/checksums.txt"

  print(string.format("kenjutu: downloading %s v%s for %s-%s...", artifact, version, arch, os_name))

  local ok, err = download(release_url("checksums.txt"), checksums_path)
  if not ok then
    error("failed to download checksums: " .. (err or "unknown error"))
  end

  local checksums_content = table.concat(vim.fn.readfile(checksums_path), "\n")
  local expected_hash = parse_checksum(checksums_content, artifact)
  if not expected_hash then
    os.remove(checksums_path)
    error("checksum not found for artifact: " .. artifact)
  end

  ok, err = download(release_url(artifact), bin_path)
  if not ok then
    os.remove(checksums_path)
    error("failed to download binary: " .. (err or "unknown error"))
  end

  if not verify_checksum(bin_path, expected_hash) then
    os.remove(bin_path)
    os.remove(checksums_path)
    error("checksum verification failed for " .. artifact .. " — binary deleted")
  end

  vim.fn.system({ "chmod", "+x", bin_path })
  os.remove(checksums_path)

  print("kenjutu: installed " .. artifact .. " v" .. version)
end

---@return string
function M.bin_path()
  local prebuilt = plugin_dir .. "/bin/kjn"
  if vim.uv.fs_stat(prebuilt) then
    return prebuilt
  end

  local source_built = plugin_dir .. "/target/release/kjn"
  if vim.uv.fs_stat(source_built) then
    return source_built
  end

  error(
    "kjn binary not found. Install with :lua require('kenjutu.install').download() or build from source with: cargo build --release --bin kjn"
  )
end

if not pcall(debug.getlocal, 4, 1) then
  M.download()
end

return M
