import { RestEndpointMethodTypes } from "@octokit/rest"
import { useMutation, useQueryClient } from "@tanstack/react-query"

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
    onSuccess: (_, { owner, repo }) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.pullRequests(owner, repo),
      })
    },
  })
}
