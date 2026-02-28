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
  start_line?: number
  original_start_line?: number
  start_side?: "LEFT" | "RIGHT"
  side: "LEFT" | "RIGHT"
  subject_type: "line" | "file"
  user: ReviewCommentUser | null
}

type OctokitReviewComment =
  RestEndpointMethodTypes["pulls"]["listReviewComments"]["response"]["data"][number]

export function toReviewComment(octokit: OctokitReviewComment): ReviewComment {
  return {
    id: octokit.id,
    body: octokit.body,
    created_at: octokit.created_at,
    updated_at: octokit.updated_at,
    original_commit_id: octokit.original_commit_id,
    commit_id: octokit.commit_id,
    path: octokit.path,
    in_reply_to_id: octokit.in_reply_to_id ?? undefined,
    line: octokit.line ?? undefined,
    original_line: octokit.original_line ?? undefined,
    start_line: octokit.start_line ?? undefined,
    original_start_line: octokit.original_start_line ?? undefined,
    start_side:
      (octokit.start_side as "LEFT" | "RIGHT" | undefined) ?? undefined,
    side: octokit.side as "LEFT" | "RIGHT",
    subject_type: octokit.subject_type as "line" | "file",
    user: {
      login: octokit.user.login,
      avatar_url: octokit.user.avatar_url,
    },
  }
}

export type ThreadedComment = {
  root: ReviewComment
  replies: ReviewComment[]
  lineNumber: number
}

export function buildCommentThreads(
  comments: ReviewComment[],
): ThreadedComment[] {
  const rootComments = comments.filter((c) => !c.in_reply_to_id)
  const replyComments = comments.filter((c) => c.in_reply_to_id)

  const threads: ThreadedComment[] = rootComments.map((root) => {
    const replies = replyComments
      .filter((reply) => reply.in_reply_to_id === root.id)
      .sort(
        (a, b) =>
          new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      )

    const lineNumber = root.line ?? root.original_line ?? 0

    return { root, replies, lineNumber }
  })

  threads.sort((a, b) => a.lineNumber - b.lineNumber)

  return threads
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
