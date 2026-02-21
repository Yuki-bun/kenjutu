import { useQueryClient } from "@tanstack/react-query"
import { useCallback } from "react"

import { commands, HunkId, ReviewStatus } from "@/bindings"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

import { DualDiffPanel } from "./DualDiff"

export function useHunkReview({
  localDir,
  commitSha,
  changeId,
  filePath,
  oldPath,
  reviewStatus,
}: {
  localDir: string
  commitSha: string
  changeId: string
  filePath: string
  oldPath: string | undefined
  reviewStatus: ReviewStatus
}) {
  const queryClient = useQueryClient()

  const invalidateAfterHunkMark = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: queryKeys.commitFileList(localDir, commitSha),
    })
    queryClient.invalidateQueries({
      queryKey: queryKeys.fileDiff(localDir, commitSha, filePath, oldPath),
    })
    queryClient.invalidateQueries({
      queryKey: queryKeys.partialReviewDiffs(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath,
      ),
    })
  }, [queryClient, localDir, commitSha, filePath, oldPath, changeId])

  const markRegionMutation = useRpcMutation({
    mutationFn: async (region: HunkId) => {
      if (!changeId) {
        throw new Error("Cannot mark region: no change ID")
      }
      return await commands.markHunkReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterHunkMark,
  })

  const unmarkRegionMutation = useRpcMutation({
    mutationFn: async (region: HunkId) => {
      return await commands.unmarkHunkReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterHunkMark,
  })

  const handleMarkRegion = useCallback(
    (region: HunkId) => {
      if (reviewStatus === "reviewed") {
        unmarkRegionMutation.mutate(region)
      } else {
        markRegionMutation.mutate(region)
      }
    },
    [reviewStatus, markRegionMutation, unmarkRegionMutation],
  )

  const handleDualMarkRegion = useCallback(
    (region: HunkId, panel: DualDiffPanel) => {
      if (panel === "remaining") {
        markRegionMutation.mutate(region)
      } else {
        unmarkRegionMutation.mutate(region)
      }
    },
    [markRegionMutation, unmarkRegionMutation],
  )

  return { handleMarkRegion, handleDualMarkRegion }
}
