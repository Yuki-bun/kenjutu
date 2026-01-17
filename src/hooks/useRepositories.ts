import { useQuery } from "@tanstack/react-query"
import { useGithub } from "@/context/GithubContext"

export interface Repo {
  id: string // node_id
  name: string
  htmlUrl: string
  ownerName: string
}

export function useRepositories() {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: ["repositories"],
    queryFn: async (): Promise<Repo[]> => {
      if (!octokit) {
        throw new Error("Not authenticated")
      }

      const { data } = await octokit.repos.listForAuthenticatedUser({
        visibility: "all",
        sort: "updated",
        per_page: 100,
      })

      return data.map((repo) => ({
        id: repo.node_id,
        name: repo.name,
        htmlUrl: repo.html_url ?? "",
        ownerName: repo.owner.login,
      }))
    },
    enabled: isAuthenticated,
  })
}
