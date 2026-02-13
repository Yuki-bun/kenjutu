import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type GitHubReview =
  RestEndpointMethodTypes["pulls"]["listReviews"]["response"]["data"][number]

export function usePullRequestReviews(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequestReviews(owner, repo, pullNumber),
    queryFn: async () => {
      const { data } = await octokit!.pulls.listReviews({
        owner,
        repo,
        pull_number: pullNumber,
      })

      return data
    },
    enabled: !!octokit && isAuthenticated,
  })
}
