import { commands } from "@/bindings"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useJjLog(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: queryKeys.jjLog(localDir),
    queryFn: () => commands.getJjLog(localDir!),
    enabled: !!localDir,
    refetchInterval: 5_000,
  })
}
