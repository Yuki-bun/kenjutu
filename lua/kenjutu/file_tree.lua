local kjn = require("kenjutu.kjn")
local jj = require("kenjutu.jj")
local utils = require("kenjutu.utils")
local file_render = require("kenjutu.file_render")

local M = {}

local ns = vim.api.nvim_create_namespace("kenjutu_file_tree")

---@param dir string
---@param commit_id string
---@param callback fun(err: string|nil, files: kenjutu.FileEntry[], metadata: kenjutu.CommitMetadata|nil)
local function fetch_commit_data(dir, commit_id, callback)
  utils.await_all({
    files = function(cb)
      kjn.files(dir, {
        commit_id = commit_id,
      }, function(err, result)
        cb(err, not err and result and result.files or nil)
      end)
    end,
    metadata = function(cb)
      jj.fetch_commit_metadata(dir, commit_id, cb)
    end,
  }, function(err, results)
    if err or not results then
      callback(err, {}, nil)
      return
    end
    callback(nil, results.files or {}, results.metadata)
  end)
end

---@param metadata kenjutu.CommitMetadata|nil
---@return kenjutu.RenderLine[]
local function format_metadata_lines(metadata)
  if not metadata then
    return {}
  end

  local lines = {} ---@type kenjutu.RenderLine[]

  if metadata.summary ~= "" then
    local text = " " .. metadata.summary
    table.insert(lines, {
      text = text,
      highlights = { { 0, #text, "KenjutuCommitSummary" } },
    })
  end

  local author_ts = {}
  local author_ts_hls = {}
  local col = 1
  table.insert(author_ts, " ")

  if metadata.author ~= "" then
    local author = metadata.author
    table.insert(author_ts, author)
    table.insert(author_ts_hls, { col, col + #author, "KenjutuCommitAuthor" })
    col = col + #author
  end

  if metadata.timestamp ~= "" then
    local sep = metadata.author ~= "" and "  " or ""
    local ts = sep .. metadata.timestamp
    table.insert(author_ts, ts)
    table.insert(author_ts_hls, { col, col + #ts, "KenjutuCommitTimestamp" })
    col = col + #ts
  end

  if #author_ts > 1 then
    table.insert(lines, {
      text = table.concat(author_ts),
      highlights = author_ts_hls,
    })
  end

  if metadata.description ~= "" then
    table.insert(lines, { text = "", highlights = {} })
    for _, desc_line in ipairs(vim.split(metadata.description, "\n", { plain = true })) do
      local text = " " .. desc_line
      table.insert(lines, {
        text = text,
        highlights = { { 0, #text, "KenjutuCommitDescription" } },
      })
    end
  end

  table.insert(lines, { text = "", highlights = {} })
  return lines
end

---@param bufnr integer
---@param files kenjutu.FileEntry[]
---@param winnr integer
---@param metadata kenjutu.CommitMetadata|nil
local function render(bufnr, files, winnr, metadata)
  local render_lines = {} ---@type kenjutu.RenderLine[]

  for _, ml in ipairs(format_metadata_lines(metadata)) do
    table.insert(render_lines, ml)
  end

  local reviewed = file_render.count_reviewed(files)
  local header = string.format(" Files %d/%d", reviewed, #files)
  table.insert(render_lines, { text = header, highlights = { { 0, #header, "KenjutuHeader" } } })
  table.insert(render_lines, { text = "", highlights = {} })

  local tree = file_render.build_tree(files)
  local tree_lines = file_render.flatten_tree(tree, #render_lines + 1)
  vim.list_extend(render_lines, tree_lines)

  file_render.apply_to_buffer(bufnr, render_lines, ns)

  if vim.api.nvim_win_is_valid(winnr) then
    vim.api.nvim_win_set_cursor(winnr, { 1, 0 })
  end
end

---@class kenjutu.FileTreeState
---@field bufnr integer
---@field winnr integer
---@field dir string
---@field current_commit_id string|nil
local FileTreeState = {}
FileTreeState.__index = FileTreeState

---@param dir string
---@param log_winnr integer
---@return kenjutu.FileTreeState
function FileTreeState.new(dir, log_winnr)
  local prev_win = vim.api.nvim_get_current_win()
  vim.api.nvim_set_current_win(log_winnr)

  vim.cmd("rightbelow vsplit")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_win_set_buf(winnr, bufnr)

  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].buflisted = false
  vim.bo[bufnr].filetype = "kenjutu-log-files"
  vim.bo[bufnr].modifiable = false

  vim.wo[winnr].cursorline = false
  vim.wo[winnr].number = false
  vim.wo[winnr].signcolumn = "no"
  vim.wo[winnr].wrap = false
  vim.wo[winnr].winfixwidth = false

  vim.cmd("wincmd =")
  vim.api.nvim_set_current_win(prev_win)

  ---@type kenjutu.FileTreeState
  local state = {
    bufnr = bufnr,
    winnr = winnr,
    dir = dir,
    current_commit_id = nil,
  }
  return setmetatable(state, FileTreeState)
end

---@param commit kenjutu.Commit
function FileTreeState:update(commit)
  if self.current_commit_id == commit.commit_id then
    return
  end
  self.current_commit_id = commit.commit_id

  local bufnr = self.bufnr
  local winnr = self.winnr

  fetch_commit_data(self.dir, commit.commit_id, function(err, files, metadata)
    if err then
      vim.notify("Failed to fetch commit data: " .. err, vim.log.levels.ERROR)
      return
    end
    if self.current_commit_id ~= commit.commit_id then
      return
    end
    render(bufnr, files, winnr, metadata)
  end)
end

function FileTreeState:close()
  if vim.api.nvim_win_is_valid(self.winnr) then
    vim.api.nvim_win_close(self.winnr, true)
  end
  if vim.api.nvim_buf_is_valid(self.bufnr) then
    vim.api.nvim_buf_delete(self.bufnr, { force = true })
  end
end

M.FileTreeState = FileTreeState

return M
