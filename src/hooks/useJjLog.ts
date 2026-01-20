import { useFailableQuery } from "@/hooks/useRpcQuery"
import { commands } from "@/bindings"

export function useJjLog(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: ["jj-log", localDir],
    queryFn: () => commands.getJjLog(localDir!),
    enabled: !!localDir,
  })
}
