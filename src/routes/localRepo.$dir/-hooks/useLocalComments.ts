import { commands, FileComments } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useLocalComments(
  localDir: string,
  changeId: string,
  sha: string,
) {
  return useRpcQuery<
    FileComments[],
    unknown,
    ReturnType<typeof queryKeys.localComments>
  >({
    queryKey: queryKeys.localComments(localDir, changeId, sha),
    queryFn: () =>
      commands.getComments({
        local_dir: localDir,
        change_id: changeId,
        sha,
      }),
  })
}
