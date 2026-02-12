import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

type GitHubIssueComment =
  RestEndpointMethodTypes["issues"]["listComments"]["response"]["data"][number]

export type PullRequestComment = {
  id: number
  author: string
  avatarUrl: string
  createdAt: string
  body: string
}

export function usePullRequestComments(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequestComments(owner, repo, pullNumber),
    queryFn: async (): Promise<PullRequestComment[]> => {
      const { data } = await octokit!.issues.listComments({
        owner,
        repo,
        issue_number: pullNumber,
      })

      return data.map((comment: GitHubIssueComment) => ({
        id: comment.id,
        author: comment.user?.login || "deleted-user",
        avatarUrl: comment.user?.avatar_url || "",
        createdAt: comment.created_at,
        body: comment.body || "",
      }))
    },
    enabled: !!octokit && isAuthenticated,
  })
}
