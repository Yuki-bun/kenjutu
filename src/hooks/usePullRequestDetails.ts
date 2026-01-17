import { useQuery } from "@tanstack/react-query"
import { useOctokit } from "@/context/GithubContext"

export interface PullRequestDetails {
  title: string
  body: string | null
  baseBranch: string
  headBranch: string
  baseSha: string
  headSha: string
  mergeable: boolean | null
}

export function usePullRequestDetails(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const octokit = useOctokit()

  return useQuery({
    queryKey: ["pullRequest", owner, repo, pullNumber],
    queryFn: async (): Promise<PullRequestDetails> => {
      const { data } = await octokit.pulls.get({
        owner,
        repo,
        pull_number: pullNumber,
      })

      return {
        title: data.title,
        body: data.body,
        baseBranch: data.base.ref,
        headBranch: data.head.ref,
        baseSha: data.base.sha,
        headSha: data.head.sha,
        mergeable: data.mergeable,
      }
    },
  })
}
