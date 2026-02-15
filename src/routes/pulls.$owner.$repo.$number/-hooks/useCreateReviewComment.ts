import { RestEndpointMethodTypes } from "@octokit/rest"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

import { type ReviewComment } from "./useReviewComments"

type CreateReviewCommentParams = {
  type: "new"
  owner: string
  repo: string
  pullNumber: number
  body: string
  commitId: string
  path: string
  line: number
  side: "LEFT" | "RIGHT"
  startLine?: number
  startSide?: "LEFT" | "RIGHT"
}

type CreateReplyParams = {
  type: "reply"
  owner: string
  repo: string
  pullNumber: number
  body: string
  inReplyTo: number
  commitId: string
  path: string
}

export type CreateCommentParams = CreateReviewCommentParams | CreateReplyParams

function createOptimisticComment(params: CreateCommentParams): ReviewComment {
  const now = new Date().toISOString()

  if (params.type === "reply") {
    return {
      id: -Date.now(),
      body: params.body,
      created_at: now,
      updated_at: now,
      commit_id: params.commitId,
      original_commit_id: params.commitId,
      path: params.path,
      in_reply_to_id: params.inReplyTo,
      line: undefined,
      original_line: undefined,
      side: "RIGHT",
      subject_type: "line",
      user: null,
    }
  }

  return {
    id: -Date.now(),
    body: params.body,
    created_at: now,
    updated_at: now,
    commit_id: params.commitId,
    original_commit_id: params.commitId,
    path: params.path,
    in_reply_to_id: undefined,
    line: params.side === "RIGHT" ? params.line : undefined,
    original_line: params.line,
    start_line: params.startLine,
    original_start_line: params.startLine,
    start_side: params.startSide,
    side: params.side,
    subject_type: "line",
    user: null,
  }
}

type CreateReviewCommentResponse =
  RestEndpointMethodTypes["pulls"]["createReviewComment"]["response"]["data"]

export function useCreateReviewComment() {
  const { octokit } = useGithub()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (
      params: CreateCommentParams,
    ): Promise<CreateReviewCommentResponse> => {
      if (!octokit) {
        throw new Error("Not authenticated")
      }

      if (params.type === "reply") {
        const { data } = await octokit.pulls.createReplyForReviewComment({
          owner: params.owner,
          repo: params.repo,
          pull_number: params.pullNumber,
          comment_id: params.inReplyTo,
          body: params.body,
        })
        return data
      }

      const { data } = await octokit.pulls.createReviewComment({
        owner: params.owner,
        repo: params.repo,
        pull_number: params.pullNumber,
        body: params.body,
        commit_id: params.commitId,
        path: params.path,
        line: params.line,
        side: params.side,
        ...(params.startLine != null && {
          start_line: params.startLine,
          start_side: params.startSide ?? params.side,
        }),
      })

      return data
    },
    onMutate: async (params) => {
      const key = queryKeys.reviewComments(
        params.owner,
        params.repo,
        params.pullNumber,
      )

      await queryClient.cancelQueries({ queryKey: key })

      const previousComments = queryClient.getQueryData<ReviewComment[]>(key)

      const optimisticComment = createOptimisticComment(params)

      queryClient.setQueryData<ReviewComment[]>(key, (old) => [
        ...(old ?? []),
        optimisticComment,
      ])

      return { previousComments }
    },
    onError: (err, params, context) => {
      if (context?.previousComments) {
        queryClient.setQueryData(
          queryKeys.reviewComments(
            params.owner,
            params.repo,
            params.pullNumber,
          ),
          context.previousComments,
        )
      }
      const message =
        err instanceof Error ? err.message : "An unexpected error occurred"
      toast.error("Failed to create comment", {
        description: message,
        position: "top-center",
        duration: 7000,
      })
    },
    onSettled: (_data, _error, params) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.reviewComments(
          params.owner,
          params.repo,
          params.pullNumber,
        ),
      })
    },
  })
}
