import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type ReviewEvent = "APPROVE" | "REQUEST_CHANGES" | "COMMENT"

export function useSubmitReview() {
  const { octokit } = useGithub()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      owner,
      repo,
      pullNumber,
      event,
      body,
    }: {
      owner: string
      repo: string
      pullNumber: number
      event: ReviewEvent
      body: string
    }) => {
      if (!octokit) {
        throw new Error("Not authenticated")
      }

      const { data } = await octokit.pulls.createReview({
        owner,
        repo,
        pull_number: pullNumber,
        event,
        body: body || undefined,
      })

      return data
    },
    onSuccess: (_result, { owner, repo, pullNumber, event }) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.pullRequestReviews(owner, repo, pullNumber),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.pullRequestComments(owner, repo, pullNumber),
      })

      const messages: Record<ReviewEvent, string> = {
        APPROVE: "Pull request approved!",
        REQUEST_CHANGES: "Changes requested.",
        COMMENT: "Review comment submitted.",
      }
      toast.success(messages[event], {
        position: "top-center",
        duration: 5000,
      })
    },
    onError: (err) => {
      const message =
        err instanceof Error
          ? err.message
          : "Failed to submit review. Please try again."
      toast.error("Review submission failed", {
        description: message,
        position: "top-center",
        duration: 7000,
      })
    },
  })
}
