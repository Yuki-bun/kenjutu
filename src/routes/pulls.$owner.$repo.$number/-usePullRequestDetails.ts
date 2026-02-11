import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useOctokit } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type PullRequestDetails =
  RestEndpointMethodTypes["pulls"]["get"]["response"]["data"]

export function usePullRequestDetails(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const octokit = useOctokit()

  return useQuery({
    queryKey: queryKeys.pullRequest(owner, repo, pullNumber),
    queryFn: async (): Promise<PullRequestDetails> => {
      const { data } = await octokit.pulls.get({
        owner,
        repo,
        pull_number: pullNumber,
      })

      return data
    },
  })
}
