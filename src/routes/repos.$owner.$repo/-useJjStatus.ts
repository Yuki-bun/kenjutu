import { commands } from "@/bindings"
import { useFailableQuery } from "@/hooks/useRpcQuery"
import { queryKeys } from "@/lib/queryKeys"

export function useJjStatus(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: queryKeys.jjStatus(localDir),
    queryFn: () => commands.getJjStatus(localDir!),
    enabled: !!localDir,
  })
}
