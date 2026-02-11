import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type Repository =
  RestEndpointMethodTypes["repos"]["get"]["response"]["data"]

export function useRepository(owner: string | null, repo: string | null) {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: queryKeys.repository(owner, repo),
    queryFn: async (): Promise<Repository> => {
      if (!octokit || !owner || !repo) {
        throw new Error("Missing required parameters")
      }

      const { data } = await octokit.repos.get({ owner, repo })
      return data
    },
    enabled: isAuthenticated && !!owner && !!repo,
  })
}
