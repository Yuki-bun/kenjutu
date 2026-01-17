import { useQuery } from "@tanstack/react-query"
import { useGithub } from "@/context/GithubContext"

export interface Repository {
  id: string // node_id
  name: string
  ownerName: string
  description: string | null
}

export function useRepository(owner: string | null, repo: string | null) {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: ["repository", owner, repo],
    queryFn: async (): Promise<Repository> => {
      if (!octokit || !owner || !repo) {
        throw new Error("Missing required parameters")
      }

      const { data } = await octokit.repos.get({ owner, repo })

      return {
        id: data.node_id,
        name: data.name,
        ownerName: data.owner.login,
        description: data.description,
      }
    },
    enabled: isAuthenticated && !!owner && !!repo,
  })
}
