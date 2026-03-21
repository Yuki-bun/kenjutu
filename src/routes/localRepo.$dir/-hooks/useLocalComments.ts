import { commands, FileComments } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useLocalComments(localDir: string, commitId: string) {
  return useRpcQuery<
    FileComments[],
    unknown,
    ReturnType<typeof queryKeys.localComments>
  >({
    queryKey: queryKeys.localComments(localDir, commitId),
    queryFn: () =>
      commands.getComments({
        local_dir: localDir,
        commit_id: commitId,
      }),
  })
}
