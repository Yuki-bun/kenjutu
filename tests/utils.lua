---@diagnostic disable: duplicate-set-field
local M = {}
local kjn = require("kenjutu.kjn")
local jj = require("kenjutu.jj")

local original_kjn_fetch_blob = kjn.fetch_blob
local original_kjn_files = kjn.files
local original_kjn_set_blob = kjn.set_blob
local original_kjn_mark_file = kjn.mark_file
local original_kjn_unmark_file = kjn.unmark_file

local original_jj_log = jj.log
local original_jj_fetch_metadata = jj.fetch_commit_metadata
local original_jj_describe = jj.describe
local original_jj_new_commit = jj.new_commit

function M.mock_all()
  kjn.fetch_blob = function(_, cb)
    cb(nil, "")
  end
  kjn.files = function(_, change_id, cb)
    cb(nil, { files = {}, commitId = "abc123", changeId = change_id })
  end
  kjn.set_blob = function(_, _, cb)
    cb(nil)
  end
  kjn.mark_file = function(_, cb)
    cb(nil)
  end
  kjn.unmark_file = function(_, cb)
    cb(nil)
  end
  jj.describe = function(_, _, _, callback)
    callback(nil)
  end
  jj.new_commit = function(_, _, callback)
    callback(nil)
  end
end

function M.restore_all()
  kjn.fetch_blob = original_kjn_fetch_blob
  kjn.files = original_kjn_files
  kjn.set_blob = original_kjn_set_blob
  kjn.mark_file = original_kjn_mark_file
  kjn.unmark_file = original_kjn_unmark_file
  jj.log = original_jj_log
  jj.fetch_commit_metadata = original_jj_fetch_metadata
  jj.describe = original_jj_describe
  jj.new_commit = original_jj_new_commit
end

return M
