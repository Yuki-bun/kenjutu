import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export interface Repo {
  id: string // node_id
  name: string
  htmlUrl: string
  ownerName: string
}

export type ListRepo =
  RestEndpointMethodTypes["repos"]["listForAuthenticatedUser"]["response"]["data"][0]

export function useRepositories() {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: queryKeys.repositories(),
    queryFn: async (): Promise<ListRepo[]> => {
      const { data } = await octokit!.repos.listForAuthenticatedUser({
        visibility: "all",
        sort: "updated",
        per_page: 100,
      })

      return data
    },
    enabled: isAuthenticated,
  })
}
