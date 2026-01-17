import { useQuery } from "@tanstack/react-query"
import { useGithub } from "@/context/GithubContext"

export interface User {
  login: string
  id: number
  avatar_url: string
  gravatar_id: string
  name: string | null
}

export interface PullRequest {
  id: number
  number: number
  title: string | null
  author: User | null
  githubUrl: string | null
}

export function usePullRequests(owner: string | null, repo: string | null) {
  const { octokit, isAuthenticated } = useGithub()

  return useQuery({
    queryKey: ["pullRequests", owner, repo],
    queryFn: async (): Promise<PullRequest[]> => {
      if (!octokit || !owner || !repo) {
        return []
      }

      const { data } = await octokit.pulls.list({
        owner,
        repo,
        state: "open",
        sort: "updated",
      })

      return data.map((pr) => ({
        id: pr.id,
        number: pr.number,
        title: pr.title,
        author: pr.user
          ? {
              login: pr.user.login,
              id: pr.user.id,
              avatar_url: pr.user.avatar_url ?? "",
              gravatar_id: pr.user.gravatar_id ?? "",
              name: pr.user.name ?? null,
            }
          : null,
        githubUrl: pr.html_url,
      }))
    },
    enabled: isAuthenticated && !!owner && !!repo,
  })
}
