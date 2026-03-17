local record M
   mock_all: function()
   restore_all: function()
   review_wins: function(): integer, integer, integer
end

local kjn = require("kenjutu.kjn") as any
local jj = require("kenjutu.jj") as any

local original_kjn_fetch_blob = kjn.fetch_blob
local original_kjn_files = kjn.files
local original_kjn_set_blob = kjn.set_blob
local original_kjn_mark_file = kjn.mark_file
local original_kjn_unmark_file = kjn.unmark_file
local original_kjn_get_comments = kjn.get_comments
local original_kjn_add_comment = kjn.add_comment
local original_kjn_edit_comment = kjn.edit_comment
local original_kjn_resolve_comment = kjn.resolve_comment
local original_kjn_unresolve_comment = kjn.unresolve_comment

local original_jj_log = jj.log
local original_jj_fetch_metadata = jj.fetch_commit_metadata
local original_jj_describe = jj.describe
local original_jj_new_commit = jj.new_commit
local original_jj_squash = jj.squash
local original_jj_list_files = jj.list_files

function M.mock_all()
  kjn.fetch_blob = function(_: any, cb: function(any, string))
    cb(nil, "")
  end
  kjn.files = function(_: any, change_id: string, cb: function(any, any))
    cb(nil, { files = {}, commitId = "abc123", changeId = change_id })
  end
  kjn.set_blob = function(_: any, _: any, cb: function(any))
    cb(nil)
  end
  kjn.mark_file = function(_: any, cb: function(any))
    cb(nil)
  end
  kjn.unmark_file = function(_: any, cb: function(any))
    cb(nil)
  end
  kjn.get_comments = function(_: any, _: any, _: any, cb: function(any, any))
    cb(nil, { files = {} })
  end
  kjn.add_comment = function(_: any, cb: function(any, any))
    cb(nil, {})
  end
  kjn.edit_comment = function(_: any, cb: function(any, any))
    cb(nil, {})
  end
  kjn.resolve_comment = function(_: any, cb: function(any))
    cb(nil)
  end
  kjn.unresolve_comment = function(_: any, cb: function(any))
    cb(nil)
  end

  jj.log = function(_: any, callback: function(any, any))
    callback(nil, { lines = {}, highlights = {}, commits_by_line = {}, commit_lines = {} })
  end
  jj.fetch_commit_metadata = function(_: any, _: any, callback: function(any, any))
    callback(nil, { summary = "", description = "", author = "", timestamp = "" })
  end
  jj.describe = function(_: any, _: any, _: any, callback: function(any))
    callback(nil)
  end
  jj.new_commit = function(_: any, _: any, callback: function(any))
    callback(nil)
  end
  jj.squash = function(_: any, _: any, callback: function(any))
    callback(nil)
  end
  jj.list_files = function(_: any, _: any, callback: function(any, any))
    callback(nil, {})
  end
end

function M.restore_all()
  kjn.fetch_blob = original_kjn_fetch_blob
  kjn.files = original_kjn_files
  kjn.set_blob = original_kjn_set_blob
  kjn.mark_file = original_kjn_mark_file
  kjn.unmark_file = original_kjn_unmark_file
  kjn.get_comments = original_kjn_get_comments
  kjn.add_comment = original_kjn_add_comment
  kjn.edit_comment = original_kjn_edit_comment
  kjn.resolve_comment = original_kjn_resolve_comment
  kjn.unresolve_comment = original_kjn_unresolve_comment

  jj.log = original_jj_log
  jj.fetch_commit_metadata = original_jj_fetch_metadata
  jj.describe = original_jj_describe
  jj.new_commit = original_jj_new_commit
  jj.squash = original_jj_squash
  jj.list_files = original_jj_list_files
end

function M.review_wins(): integer, integer, integer
  local layout = vim.fn.winlayout() as {any}
  assert(layout[1] == "row", "expected row layout, got " .. (layout[1] as string))
  local children = layout[2] as {{any}}
  assert(#children == 3, "expected 3 children (file list, diff left, diff right), got " .. #children)
  local file_list_winnr = children[1][2] as integer
  local diff_left = children[2][2] as integer
  local diff_right = children[3][2] as integer
  assert(type(file_list_winnr) == "number", "expected file list leaf")
  assert(type(diff_left) == "number", "expected diff left leaf")
  assert(type(diff_right) == "number", "expected diff right leaf")
  return file_list_winnr, diff_left, diff_right
end

return M
