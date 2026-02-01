import { commands } from "@/bindings"
import { useFailableQuery } from "@/hooks/useRpcQuery"

export function useJjStatus(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: ["jj-status", localDir],
    queryFn: () => commands.getJjStatus(localDir!),
    enabled: !!localDir,
  })
}
