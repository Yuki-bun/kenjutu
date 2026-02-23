import { commands } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useJjLogGraph(localDir: string | undefined) {
  return useRpcQuery({
    queryKey: queryKeys.jjLog(localDir),
    queryFn: () => commands.getJjLog(localDir!),
    enabled: !!localDir,
    refetchInterval: 5_000,
  })
}
