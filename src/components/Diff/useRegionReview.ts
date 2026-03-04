import { useQueryClient } from "@tanstack/react-query"
import { useCallback } from "react"

import { commands, RegionId } from "@/bindings"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

import { DualDiffPanel } from "./DualDiff"

export function useRegionReview({
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

  const invalidateAfterRegionMark = useCallback(() => {
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
    mutationFn: async (region: RegionId) => {
      return await commands.markRegionReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterRegionMark,
  })

  const unmarkRegionMutation = useRpcMutation({
    mutationFn: async (region: RegionId) => {
      return await commands.unmarkRegionReviewed(
        localDir,
        changeId,
        commitSha,
        filePath,
        oldPath ?? null,
        region,
      )
    },
    onSuccess: invalidateAfterRegionMark,
  })

  const handleDualMarkRegion = useCallback(
    (region: RegionId, panel: DualDiffPanel) => {
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
