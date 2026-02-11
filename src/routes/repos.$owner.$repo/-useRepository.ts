import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type Repository =
  RestEndpointMethodTypes["repos"]["get"]["response"]["data"]

export function useRepository(owner: string, repo: string) {
  const { octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.repository(owner, repo),
    queryFn: async (): Promise<Repository> => {
      const { data } = await octokit!.repos.get({ owner, repo })
      return data
    },
    enabled: !octokit,
  })
}
