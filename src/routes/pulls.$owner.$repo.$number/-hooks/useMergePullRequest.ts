import { RestEndpointMethodTypes } from "@octokit/rest"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export type MergeResult =
  RestEndpointMethodTypes["pulls"]["merge"]["response"]["data"]

export function useMergePullRequest() {
  const { octokit } = useGithub()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      owner,
      repo,
      pullNumber,
    }: {
      owner: string
      repo: string
      pullNumber: number
    }): Promise<MergeResult> => {
      if (!octokit) {
        throw new Error("Not authenticated")
      }

      const { data } = await octokit.pulls.merge({
        owner,
        repo,
        pull_number: pullNumber,
        merge_method: "rebase",
      })

      return data
    },
    onSuccess: (result, { owner, repo, pullNumber }) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.pr(owner, repo, pullNumber),
      })
      toast.success("Pull request merged successfully!", {
        description: `SHA: ${result.sha}`,
        position: "top-center",
        duration: 5000,
      })
    },
    onError: (err) => {
      const message =
        err instanceof Error
          ? err.message
          : "Failed to merge pull request. Please try again."
      toast.error("Merge failed", {
        description: message,
        position: "top-center",
        duration: 7000,
      })
    },
  })
}
