import { commands } from "@/bindings"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useCommitFileList(
  localDir: string,
  commitSha: string | undefined,
) {
  return useFailableQuery({
    queryKey: queryKeys.commitFileList(localDir, commitSha ?? ""),
    queryFn: () => commands.getCommitFileList(localDir, commitSha!),
    enabled: !!commitSha,
  })
}
