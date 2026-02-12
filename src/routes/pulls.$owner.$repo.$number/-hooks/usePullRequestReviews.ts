import { RestEndpointMethodTypes } from "@octokit/rest"
import { useQuery } from "@tanstack/react-query"

import { useGithub } from "@/context/GithubContext"
import { queryKeys } from "@/lib/queryKeys"

type ReviewStatus = "approved" | "changes_requested" | "pending" | "commented"

export type Reviewer = {
  username: string
  avatarUrl?: string
  status: ReviewStatus
}

type GitHubReview =
  RestEndpointMethodTypes["pulls"]["listReviews"]["response"]["data"][number]

function mapGitHubStateToStatus(state: string): ReviewStatus {
  switch (state) {
    case "APPROVED":
      return "approved"
    case "CHANGES_REQUESTED":
      return "changes_requested"
    case "COMMENTED":
      return "commented"
    case "DISMISSED":
      return "commented"
    case "PENDING":
      return "pending"
    default:
      return "commented"
  }
}

export function usePullRequestReviews(
  owner: string,
  repo: string,
  pullNumber: number,
) {
  const { isAuthenticated, octokit } = useGithub()

  return useQuery({
    queryKey: queryKeys.pullRequestReviews(owner, repo, pullNumber),
    queryFn: async (): Promise<Reviewer[]> => {
      const [{ data: reviews }, { data: pullRequest }] = await Promise.all([
        octokit!.pulls.listReviews({
          owner,
          repo,
          pull_number: pullNumber,
        }),
        octokit!.pulls.get({
          owner,
          repo,
          pull_number: pullNumber,
        }),
      ])

      const reviewsByUser = new Map<string, GitHubReview>()

      const sortedReviews = [...reviews].sort((a, b) => {
        if (!a.submitted_at || !b.submitted_at) return 0
        return (
          new Date(b.submitted_at).getTime() -
          new Date(a.submitted_at).getTime()
        )
      })

      for (const review of sortedReviews) {
        if (!review.user?.login) continue

        const username = review.user.login
        if (!reviewsByUser.has(username)) {
          if (review.state !== "DISMISSED") {
            reviewsByUser.set(username, review)
          } else if (
            !sortedReviews.some(
              (r) => r.user?.login === username && r.state !== "DISMISSED",
            )
          ) {
            reviewsByUser.set(username, review)
          }
        }
      }

      const reviewers: Reviewer[] = Array.from(reviewsByUser.values()).map(
        (review) => ({
          username: review.user!.login,
          avatarUrl: review.user!.avatar_url,
          status: mapGitHubStateToStatus(review.state),
        }),
      )

      if (pullRequest.requested_reviewers) {
        for (const requested of pullRequest.requested_reviewers) {
          if (!reviewsByUser.has(requested.login)) {
            reviewers.push({
              username: requested.login,
              avatarUrl: requested.avatar_url,
              status: "pending",
            })
          }
        }
      }

      return reviewers
    },
    enabled: !!octokit && isAuthenticated,
  })
}
