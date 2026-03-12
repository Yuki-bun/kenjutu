local M = {}
local kjn = require("kenjutu.kjn")

local original_kjn_fetch_blob = kjn.fetch_blob
local original_kjn_files = kjn.files
local original_kjn_set_blob = kjn.set_blob
local original_kjn_mark_file = kjn.mark_file
local original_kjn_unmark_file = kjn.unmark_file

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
end

function M.restore_all()
  kjn.fetch_blob = original_kjn_fetch_blob
  kjn.files = original_kjn_files
  kjn.set_blob = original_kjn_set_blob
  kjn.mark_file = original_kjn_mark_file
  kjn.unmark_file = original_kjn_unmark_file
end

return M
