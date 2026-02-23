import { useQueryClient } from "@tanstack/react-query"
import { useCallback } from "react"

import { commands, HunkId } from "@/bindings"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

import { DualDiffPanel } from "./DualDiff"

export function useHunkReview({
  localDir,
  commitSha,
  changeId,
  filePath,
  oldPath,
}: {
  localDir: string
  commitSha: string
  changeId: string
  filePath: string
  oldPath: string | undefined
}) {
  const queryClient = useQueryClient()

  const invalidateAfterHunkMark = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: queryKeys.commitFileList(localDir, commitSha),
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

  return { handleDualMarkRegion }
}
