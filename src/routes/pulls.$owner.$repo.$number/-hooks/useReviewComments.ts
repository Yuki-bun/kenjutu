import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type GithubReviewComment =
  RestEndpointMethodTypes["pulls"]["listReviewComments"]["response"]["data"][number]

export function useReviewComments(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.reviewComments(owner, repo, pullNumber),
    queryFn: async () => {
      const { data } = await octokit!.pulls.listReviewComments({
        owner,
        repo,
        pull_number: pullNumber,
      })

      return data
    },
    enabled: !!octokit && isAuthenticated,
  })
}
