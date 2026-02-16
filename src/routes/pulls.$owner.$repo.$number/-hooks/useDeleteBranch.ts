import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

export function useDeleteBranch({
  owner,
  repo,
  branch,
}: {
  owner: string
  repo: string
  branch: string | undefined
}) {
  const { octokit } = useGithub()
  const queryClient = useQueryClient()

  const branchKey = queryKeys.branch(owner, repo, branch!)

  const { data: branchExists } = useQuery({
    queryKey: branchKey,
    queryFn: async () => {
      if (!octokit) throw new Error("Not authenticated")
      await octokit.git.getRef({ ref: `heads/${branch}`, owner, repo })
      return true
    },
    enabled: !!octokit && !!branch,
  })

  const mutation = useMutation({
    mutationFn: async () => {
      if (!octokit) throw new Error("Not authenticated")
      await octokit.git.deleteRef({ ref: `heads/${branch}`, owner, repo })
    },
    onMutate: () => {
      queryClient.setQueryData(branchKey, null)
    },
    onError: (err) => {
      toast.error("Delete failed", {
        description: err instanceof Error ? err.message : "Please try again.",
        position: "top-center",
        duration: 7000,
      })
    },
    onSuccess: () => {
      toast.success(`Branch '${branch}' deleted successfully!`, {
        position: "top-center",
        duration: 5000,
      })
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: branchKey })
    },
  })

  return branchExists ? mutation : null
}
