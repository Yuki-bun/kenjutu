export const queryKeys = {
  repositories: () => ["repositories"] as const,
  localRepos: () => ["local_repos"] as const,
  repository: (owner: string | null, repo: string | null) =>
    ["repository", owner, repo] as const,
  localRepoPath: (id: string) => ["localRepoPath", id] as const,
  pullRequests: (owner: string | null, repo: string | null) =>
    ["pullRequests", owner, repo] as const,
  pullRequest: (owner: string, repo: string, pullNumber: number) =>
    ["pullRequest", owner, repo, pullNumber] as const,
  pullRequestReviews: (owner: string, repo: string, pullNumber: number) =>
    ["pullRequestReviews", owner, repo, pullNumber] as const,
  pullRequestComments: (owner: string, repo: string, pullNumber: number) =>
    ["pullRequestComments", owner, repo, pullNumber] as const,
  pullRequestChecks: (owner: string, repo: string, ref: string) =>
    ["pullRequestChecks", owner, repo, ref] as const,
  pullRequestCommits: (
    localDir: string | null,
    baseSha: string | undefined,
    headSha: string | undefined,
  ) => ["pullRequestCommits", localDir, baseSha, headSha] as const,
  commitFileList: (localDir: string, commitSha: string) =>
    ["commit-file-list", localDir, commitSha] as const,
  fileDiff: (
    localDir: string,
    commitSha: string,
    filePath: string,
    oldPath?: string,
  ) => ["file-diff", localDir, commitSha, filePath, oldPath] as const,
  jjLog: (localDir: string | undefined) => ["jj-log", localDir] as const,
  jjStatus: (localDir: string | undefined) => ["jj-status", localDir] as const,
}
