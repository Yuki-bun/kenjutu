import { commands } from "@/bindings"
import { useFailableQuery } from "@/hooks/useRpcQuery"

export function useJjLog(localDir: string | undefined) {
  return useFailableQuery({
    queryKey: ["jj-log", localDir],
    queryFn: () => commands.getJjLog(localDir!),
    enabled: !!localDir,
  })
}
