import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"

export interface User {
  login: string
  id: number
  avatar_url: string
  gravatar_id: string
  name: string | null
}

export type PullRequests =
  RestEndpointMethodTypes["pulls"]["list"]["response"]["data"]

export function usePullRequests(owner: string | null, repo: string | null) {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: ["pullRequests", owner, repo],
    queryFn: async (): Promise<PullRequests> => {
      if (!octokit || !owner || !repo) {
        return []
      }

      const { data } = await octokit.pulls.list({
        owner,
        repo,
        state: "open",
        sort: "updated",
      })
      return data
    },
    enabled: isAuthenticated && !!owner && !!repo,
  })
}
