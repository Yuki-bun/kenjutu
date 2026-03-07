local MiniTest = require("mini.test")
local expect = MiniTest.expect
local h = require("tests.e2e.helpers")

local kjn = require("kenjutu.kjn")

local T = MiniTest.new_set()

local repo

T["e2e kjn"] = MiniTest.new_set({
  hooks = {
    pre_case = function()
      repo = h.create_repo()
    end,
    post_case = function()
      if repo then
        repo.cleanup()
        repo = nil
      end
    end,
  },
})

T["e2e kjn"]["files returns file list for added file"] = function()
  h.write_file(repo, "hello.txt", "hello world\n")
  local commit = h.jj_commit(repo, "add hello")

  local err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)

  expect.equality(err, nil)
  expect.no_equality(result, nil)
  expect.equality(type(result.commitId), "string")
  expect.equality(type(result.files), "table")
  expect.equality(#result.files, 1)
  expect.equality(result.files[1].newPath, "hello.txt")
  expect.equality(result.files[1].status, "added")
  expect.equality(result.files[1].additions > 0, true)
end

T["e2e kjn"]["files shows modified status"] = function()
  h.write_file(repo, "file.txt", "version 1\n")
  h.jj_commit(repo, "add file")

  h.write_file(repo, "file.txt", "version 2\n")
  local commit = h.jj_commit(repo, "modify file")

  local err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)

  expect.equality(err, nil)
  expect.no_equality(result, nil)
  expect.equality(#result.files, 1)
  expect.equality(result.files[1].status, "modified")
end

T["e2e kjn"]["files shows deleted status"] = function()
  h.write_file(repo, "gone.txt", "bye\n")
  h.jj_commit(repo, "add file")

  h.delete_file(repo, "gone.txt")
  local commit = h.jj_commit(repo, "delete file")

  local err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)

  expect.equality(err, nil)
  expect.no_equality(result, nil)
  expect.equality(#result.files, 1)
  expect.equality(result.files[1].status, "deleted")
end

T["e2e kjn"]["files shows multiple files"] = function()
  h.write_file(repo, "a.txt", "a\n")
  h.write_file(repo, "b.txt", "b\n")
  h.write_file(repo, "src/c.txt", "c\n")
  local commit = h.jj_commit(repo, "add three files")

  local err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)

  expect.equality(err, nil)
  expect.equality(#result.files, 3)
end

T["e2e kjn"]["blob returns target file content"] = function()
  h.write_file(repo, "content.txt", "expected content\n")
  local commit = h.jj_commit(repo, "add content")

  local err, content = h.sync(function(cb)
    kjn.fetch_blob({
      tree_kind = "target",
      change_id = commit.change_id,
      commit_id = commit.commit_id,
      file_path = "content.txt",
      dir = repo.path,
    }, cb)
  end)

  expect.equality(err, nil)
  expect.equality(content, "expected content\n")
end

T["e2e kjn"]["blob base returns parent version"] = function()
  h.write_file(repo, "evolving.txt", "version 1\n")
  h.jj_commit(repo, "v1")

  h.write_file(repo, "evolving.txt", "version 2\n")
  local commit = h.jj_commit(repo, "v2")

  local err, content = h.sync(function(cb)
    kjn.fetch_blob({
      tree_kind = "base",
      change_id = commit.change_id,
      commit_id = commit.commit_id,
      file_path = "evolving.txt",
      dir = repo.path,
    }, cb)
  end)

  expect.equality(err, nil)
  expect.equality(content, "version 1\n")
end

T["e2e kjn"]["mark-file changes review status to reviewed"] = function()
  h.write_file(repo, "review-me.txt", "content\n")
  local commit = h.jj_commit(repo, "add file to review")

  -- Initially unreviewed
  local err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)
  expect.equality(err, nil)
  expect.equality(result.files[1].reviewStatus, "unreviewed")

  -- Mark as reviewed
  err = h.sync(function(cb)
    kjn.run(repo.path, {
      "mark-file",
      "--change-id",
      commit.change_id,
      "--commit",
      commit.commit_id,
      "--file",
      "review-me.txt",
    }, cb)
  end)
  expect.equality(err, nil)

  -- Verify reviewed
  err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)
  expect.equality(err, nil)
  expect.equality(result.files[1].reviewStatus, "reviewed")
end

T["e2e kjn"]["unmark-file reverts review status"] = function()
  h.write_file(repo, "toggle.txt", "content\n")
  local commit = h.jj_commit(repo, "add file")

  -- Mark
  h.sync(function(cb)
    kjn.run(repo.path, {
      "mark-file",
      "--change-id",
      commit.change_id,
      "--commit",
      commit.commit_id,
      "--file",
      "toggle.txt",
    }, cb)
  end)

  -- Unmark
  local err = h.sync(function(cb)
    kjn.run(repo.path, {
      "unmark-file",
      "--change-id",
      commit.change_id,
      "--commit",
      commit.commit_id,
      "--file",
      "toggle.txt",
    }, cb)
  end)
  expect.equality(err, nil)

  -- Verify no longer reviewed
  err, result = h.sync(function(cb)
    kjn.run(repo.path, { "files", "--change-id", commit.change_id }, cb)
  end)
  expect.equality(err, nil)
  expect.no_equality(result.files[1].reviewStatus, "reviewed")
end

T["e2e kjn"]["set-blob updates marker tree content"] = function()
  h.write_file(repo, "marker-test.txt", "original\n")
  local commit = h.jj_commit(repo, "add file")

  local marker_content = "partially reviewed\n"
  local err = h.sync(function(cb)
    kjn.run_with_stdin(repo.path, {
      "set-blob",
      "--change-id",
      commit.change_id,
      "--commit",
      commit.commit_id,
      "--file",
      "marker-test.txt",
    }, marker_content, cb)
  end)
  expect.equality(err, nil)

  -- Read back marker blob
  local content
  err, content = h.sync(function(cb)
    kjn.fetch_blob({
      tree_kind = "marker",
      change_id = commit.change_id,
      commit_id = commit.commit_id,
      file_path = "marker-test.txt",
      dir = repo.path,
    }, cb)
  end)
  expect.equality(err, nil)
  expect.equality(content, marker_content)
end

return T
