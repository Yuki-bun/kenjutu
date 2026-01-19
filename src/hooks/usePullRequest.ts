import { useQuery, useQueryClient } from "@tanstack/react-query"
import { commands } from "@/bindings"
import { usePullRequestDetails } from "./usePullRequestDetails"

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
  repoId: string,
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
  } = useQuery({
    queryKey: [
      "pullRequestCommits",
      repoId,
      prDetails?.baseSha,
      prDetails?.headSha,
    ],
    queryFn: async () => {
      const result = await commands.getCommitsInRange(
        repoId,
        prDetails!.baseSha,
        prDetails!.headSha,
      )
      if (result.status === "error") {
        throw result.error
      }
      return result.data
    },
    enabled: !!prDetails && !!prDetails.baseSha && !!prDetails.headSha,
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
