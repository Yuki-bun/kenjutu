import { useQueryClient } from "@tanstack/react-query"
import { useCallback } from "react"

import { commands, DiffSide } from "@/bindings"
import { useRpcMutation } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

function mapSide(side: "LEFT" | "RIGHT"): DiffSide {
  return side === "LEFT" ? "Old" : "New"
}

export function useLocalCommentMutations(
  localDir: string,
  changeId: string,
  sha: string,
) {
  const queryClient = useQueryClient()

  const invalidate = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: queryKeys.localComments(localDir, changeId, sha),
    })
  }, [queryClient, localDir, changeId, sha])

  const addComment = useRpcMutation({
    mutationFn: (params: {
      filePath: string
      side: "LEFT" | "RIGHT"
      line: number
      startLine?: number
      body: string
    }) =>
      commands.addComment({
        local_dir: localDir,
        change_id: changeId,
        sha,
        file_path: params.filePath,
        side: mapSide(params.side),
        line: params.line,
        start_line: params.startLine ?? null,
        body: params.body,
      }),
    onSuccess: invalidate,
  })

  const replyToComment = useRpcMutation({
    mutationFn: (params: {
      filePath: string
      parentCommentId: string
      body: string
    }) =>
      commands.replyToComment({
        local_dir: localDir,
        change_id: changeId,
        sha,
        file_path: params.filePath,
        parent_comment_id: params.parentCommentId,
        body: params.body,
      }),
    onSuccess: invalidate,
  })

  const editComment = useRpcMutation({
    mutationFn: (params: {
      filePath: string
      commentId: string
      body: string
    }) =>
      commands.editComment({
        local_dir: localDir,
        change_id: changeId,
        sha,
        file_path: params.filePath,
        comment_id: params.commentId,
        body: params.body,
      }),
    onSuccess: invalidate,
  })

  const resolveComment = useRpcMutation({
    mutationFn: (params: { filePath: string; commentId: string }) =>
      commands.resolveComment({
        local_dir: localDir,
        change_id: changeId,
        sha,
        file_path: params.filePath,
        comment_id: params.commentId,
      }),
    onSuccess: invalidate,
  })

  const unresolveComment = useRpcMutation({
    mutationFn: (params: { filePath: string; commentId: string }) =>
      commands.unresolveComment({
        local_dir: localDir,
        change_id: changeId,
        sha,
        file_path: params.filePath,
        comment_id: params.commentId,
      }),
    onSuccess: invalidate,
  })

  return {
    addComment,
    replyToComment,
    editComment,
    resolveComment,
    unresolveComment,
  }
}
