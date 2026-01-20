import { useFailableQuery } from "@/hooks/useRpcQuery"
import { commands } from "@/bindings"

export function useJjStatus(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: ["jj-status", localDir],
    queryFn: () => commands.getJjStatus(localDir!),
    enabled: !!localDir,
  })
}
