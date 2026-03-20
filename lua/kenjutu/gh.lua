---@class kenjutu.GhAuthor
---@field login string
---@field name string

---@class kenjutu.GhCommit
---@field oid string
---@field messageHeadline string
---@field messageBody string
---@field authoredDate string
---@field authors kenjutu.GhAuthor[]

---@class kenjutu.GhCheckRun
---@field name string
---@field status string
---@field conclusion string
---@field startedAt string
---@field completedAt string

---@class kenjutu.GhPullRequest
---@field number integer
---@field title string
---@field body string
---@field author kenjutu.GhAuthor
---@field headRefName string
---@field baseRefName string
---@field isDraft boolean
---@field reviewDecision string
---@field additions integer
---@field deletions integer
---@field changedFiles integer
---@field createdAt string
---@field commits kenjutu.GhCommit[]
---@field statusCheckRollup kenjutu.GhCheckRun[]

local M = {}

local PR_FIELDS = table.concat({
  "number",
  "title",
  "body",
  "author",
  "headRefName",
  "baseRefName",
  "isDraft",
  "reviewDecision",
  "additions",
  "deletions",
  "changedFiles",
  "createdAt",
  "commits",
  "statusCheckRollup",
}, ",")

---@param dir string
---@param cb fun(err: string|nil, prs: kenjutu.GhPullRequest[]|nil)
function M.list_prs(dir, cb)
  vim.system(
    { "gh", "pr", "list", "--json", PR_FIELDS, "--limit", "30" },
    { cwd = dir, text = true },
    vim.schedule_wrap(function(obj)
      if obj.code ~= 0 then
        cb(vim.trim(obj.stderr or "gh pr list failed"), nil)
        return
      end
      local ok, decoded = pcall(vim.json.decode, obj.stdout or "[]")
      if not ok then
        cb("Failed to parse gh output: " .. tostring(decoded), nil)
        return
      end
      cb(nil, decoded)
    end)
  )
end

return M
