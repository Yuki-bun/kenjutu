import { keepPreviousData } from "@tanstack/react-query"

import { commands } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useCommitFileList(
  localDir: string,
  commitSha: string | undefined,
) {
  return useRpcQuery({
    placeholderData: keepPreviousData,
    queryKey: queryKeys.commitFileList(localDir, commitSha ?? ""),
    queryFn: () => commands.getCommitFileList(localDir, commitSha!),
    enabled: !!commitSha,
  })
}
