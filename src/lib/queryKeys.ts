export const queryKeys = {
  repositories: () => ["repositories"] as const,
  localRepos: () => ["local_repos"] as const,
  repository: (owner: string | null, repo: string | null) =>
    ["repository", owner, repo] as const,
  branch: (owner: string, repo: string, branch: string) =>
    ["branch", owner, repo, branch] as const,
  localRepoPath: (id: string) => ["localRepoPath", id] as const,
  pullRequests: (owner: string | null, repo: string | null) =>
    ["pullRequests", owner, repo] as const,
  pr: (owner: string, repo: string, pullNumber: number) =>
    ["pr", owner, repo, pullNumber] as const,
  pullRequest: (owner: string, repo: string, pullNumber: number) =>
    ["pr", owner, repo, pullNumber, "details"] as const,
  pullRequestReviews: (owner: string, repo: string, pullNumber: number) =>
    ["pr", owner, repo, pullNumber, "reviews"] as const,
  pullRequestComments: (owner: string, repo: string, pullNumber: number) =>
    ["pr", owner, repo, pullNumber, "comments"] as const,
  reviewComments: (owner: string, repo: string, pullNumber: number) =>
    ["pr", owner, repo, pullNumber, "reviewComments"] as const,
  pullRequestChecks: (owner: string, repo: string, ref: string) =>
    ["pr", owner, repo, "checks", ref] as const,
  commitsInRange: (
    localDir: string | null,
    baseSha: string | undefined,
    headSha: string | undefined,
  ) => ["pullRequestCommits", localDir, baseSha, headSha] as const,
  commitFileList: (localDir: string, commitSha: string) =>
    ["commit-file-list", localDir, commitSha] as const,
  partialReviewDiffs: (
    localDir: string,
    changeId: string,
    commitSha: string,
    filePath: string,
    oldPath?: string,
  ) =>
    [
      "partial-review-diffs",
      localDir,
      changeId,
      commitSha,
      filePath,
      oldPath,
    ] as const,
  changeIdFromSha: (localDir: string, sha: string) =>
    ["change-id-from-sha", localDir, sha] as const,
  jjLog: (localDir: string | undefined) => ["jj-log", localDir] as const,
  jjStatus: (localDir: string | undefined) => ["jj-status", localDir] as const,
  localComments: (localDir: string, changeId: string, sha: string) =>
    ["local-comments", localDir, changeId, sha] as const,
  sshSettings: () => ["ssh-settings"] as const,
}
