import { useShaToChangeId } from "@/context/ShaToChangeIdContext"

import { PRCommit } from "../routes/pulls.$owner.$repo.$number/-hooks/usePullRequest"
import { GithubReviewComment } from "../routes/pulls.$owner.$repo.$number/-hooks/useReviewComments"

/**
 * Filter review comments for a specific commit and file.
 * Uses SHA and change_id matching to handle rebased PRs.
 */
export function useCommitComments(
  currentCommit: PRCommit,
  filePath: string,
  allReviewComments: GithubReviewComment[],
): GithubReviewComment[] {
  const { getChangeId } = useShaToChangeId()
  const matchesCurrentCommit = (comment: GithubReviewComment): boolean => {
    if (comment.original_commit_id === currentCommit.sha) return true

    const commentChangeId = getChangeId(comment.original_commit_id)
    return (
      commentChangeId != null &&
      currentCommit.changeId != null &&
      commentChangeId === currentCommit.changeId
    )
  }

  return allReviewComments.filter(
    (comment) => matchesCurrentCommit(comment) && comment.path === filePath,
  )
}
