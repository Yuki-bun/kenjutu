import { useMutation, useQueryClient } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"

export interface MergeResult {
  sha: string
  merged: boolean
  message: string
}

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
        merge_method: "squash",
      })

      return {
        sha: data.sha ?? "",
        merged: data.merged,
        message: data.message,
      }
    },
    onSuccess: (_, { owner, repo }) => {
      queryClient.invalidateQueries({ queryKey: ["pullRequests", owner, repo] })
    },
  })
}
