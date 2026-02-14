import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export interface ReviewCommentUser {
  login: string
  avatar_url: string
}

export interface ReviewComment {
  id: number
  body: string
  created_at: string
  updated_at: string
  original_commit_id: string
  commit_id: string
  path: string
  in_reply_to_id?: number
  line?: number
  original_line?: number
  side: "LEFT" | "RIGHT"
  subject_type: "line" | "file"
  user: ReviewCommentUser | null
}

type OctokitReviewComment =
  RestEndpointMethodTypes["pulls"]["listReviewComments"]["response"]["data"][number]

export function toReviewComment(octokit: OctokitReviewComment): ReviewComment {
  return {
    id: octokit.id,
    body: octokit.body ?? "",
    created_at: octokit.created_at,
    updated_at: octokit.updated_at,
    original_commit_id: octokit.original_commit_id,
    commit_id: octokit.commit_id,
    path: octokit.path,
    in_reply_to_id: octokit.in_reply_to_id ?? undefined,
    line: octokit.line ?? undefined,
    original_line: octokit.original_line ?? undefined,
    side: octokit.side as "LEFT" | "RIGHT",
    subject_type: (octokit.subject_type ?? "line") as "line" | "file",
    user: octokit.user
      ? {
          login: octokit.user.login,
          avatar_url: octokit.user.avatar_url,
        }
      : null,
  }
}

export function useReviewComments(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.reviewComments(owner, repo, pullNumber),
    queryFn: async (): Promise<ReviewComment[]> => {
      const { data } = await octokit!.pulls.listReviewComments({
        owner,
        repo,
        pull_number: pullNumber,
        per_page: 100,
      })

      return data.map(toReviewComment)
    },
    enabled: !!octokit && isAuthenticated,
  })
}
