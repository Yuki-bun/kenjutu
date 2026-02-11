import { commands } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

import { usePullRequestDetails } from "./-usePullRequestDetails"

export interface PRCommit {
  changeId: string | null
  sha: string
  summary: string
  description: string
}

export interface PullRequest {
  title: string
  body: string | null
  baseBranch: string
  headBranch: string
  baseSha: string
  headSha: string
  mergeable: boolean | null
  commits: PRCommit[]
}

export function usePullRequest(
  localDir: string | null,
  owner: string,
  repo: string,
  prNumber: number,
) {
  const {
    data: prDetails,
    isLoading: detailsLoading,
    error: detailsError,
    refetch,
  } = usePullRequestDetails(owner, repo, prNumber)

  const baseSha = prDetails?.base.sha
  const headSha = prDetails?.head.sha

  const {
    data: commits,
    isLoading: commitsLoading,
    error: commitsError,
  } = useRpcQuery({
    queryKey: queryKeys.pullRequestCommits(localDir, baseSha, headSha),
    queryFn: () => commands.getCommitsInRange(localDir!, baseSha!, headSha!),
    enabled: !!localDir && !!prDetails && !!baseSha && !!headSha,
  })

  return {
    data:
      prDetails && commits
        ? {
            ...prDetails,
            commits,
          }
        : null,
    isLoading: detailsLoading || commitsLoading,
    error: detailsError || commitsError,
    refetch,
  }
}
