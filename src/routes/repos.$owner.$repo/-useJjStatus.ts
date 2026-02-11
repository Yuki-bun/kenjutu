import { commands } from "@/bindings"
import { useRpcQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useJjStatus(localDir: string | undefined) {
  return useRpcQuery({
    queryKey: queryKeys.jjStatus(localDir),
    queryFn: () => commands.getJjStatus(localDir!),
    enabled: !!localDir,
  })
}
