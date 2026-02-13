import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type GitHubIssueComment =
  RestEndpointMethodTypes["issues"]["listComments"]["response"]["data"][number]

export function usePullRequestComments(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequestComments(owner, repo, pullNumber),
    queryFn: async () => {
      const { data } = await octokit!.issues.listComments({
        owner,
        repo,
        issue_number: pullNumber,
      })

      return data
    },
    enabled: !!octokit && isAuthenticated,
  })
}
