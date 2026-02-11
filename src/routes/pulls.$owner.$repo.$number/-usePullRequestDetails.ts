import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type PullRequestDetails =
  RestEndpointMethodTypes["pulls"]["get"]["response"]["data"]

export function usePullRequestDetails(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequest(owner, repo, pullNumber),
    queryFn: async (): Promise<PullRequestDetails> => {
      const { data } = await octokit!.pulls.get({
        owner,
        repo,
        pull_number: pullNumber,
      })

      return data
    },
    enabled: !!octokit && isAuthenticated,
  })
}
