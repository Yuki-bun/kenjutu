import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export interface User {
  login: string
  id: number
  avatar_url: string
  gravatar_id: string
  name: string | null
}

export type PullRequests =
  RestEndpointMethodTypes["pulls"]["list"]["response"]["data"]

export function usePullRequests(owner: string, repo: string) {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequests(owner, repo),
    queryFn: async (): Promise<PullRequests> => {
      const { data } = await octokit!.pulls.list({
        owner,
        repo,
        state: "open",
        sort: "updated",
      })
      return data
    },
    enabled: isAuthenticated && !!octokit,
  })
}
