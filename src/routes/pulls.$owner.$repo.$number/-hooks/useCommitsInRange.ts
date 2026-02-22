import { commands } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useCommitsInRange(
  localDir: string | null,
  baseSha: string | undefined,
  headSha: string | undefined,
  remoteUrls: string[],
) {
  return useRpcQuery({
    queryKey: queryKeys.commitsInRange(localDir, baseSha, headSha),
    queryFn: () =>
      commands.getCommitsInRange(localDir!, baseSha!, headSha!, remoteUrls),
    enabled: !!localDir && !!baseSha && !!headSha,
  })
}
