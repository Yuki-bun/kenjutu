import { commands } from "@/bindings"
import { usePullRequestDetails } from "./usePullRequestDetails"
import { useFailableQuery } from "./useRpcQuery"

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

  const {
    data: commits,
    isLoading: commitsLoading,
    error: commitsError,
  } = useFailableQuery({
    queryKey: [
      "pullRequestCommits",
      localDir,
      prDetails?.baseSha,
      prDetails?.headSha,
    ],
    queryFn: () =>
      commands.getCommitsInRange(
        localDir!,
        prDetails!.baseSha,
        prDetails!.headSha,
      ),
    enabled:
      !!localDir && !!prDetails && !!prDetails.baseSha && !!prDetails.headSha,
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
