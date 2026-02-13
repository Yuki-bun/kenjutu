import { useQueryClient } from "@tanstack/react-query"
import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react"

import { commands } from "@/bindings"
import { queryKeys } from "@/lib/queryKeys"

type ShaToChangeIdContextValue = {
  getChangeId: (sha: string) => string | null | undefined
}

const ShaToChangeIdContext = createContext<ShaToChangeIdContextValue | null>(
  null,
)

type ShaToChangeIdProviderProps = {
  localDir: string
  children: ReactNode
}

export function ShaToChangeIdProvider({
  localDir,
  children,
}: ShaToChangeIdProviderProps) {
  const queryClient = useQueryClient()

  const [cache, setCache] = useState<Map<string, string | null>>(new Map())

  const getChangeId = useCallback(
    (sha: string) => {
      if (cache.has(sha)) {
        return cache.get(sha)
      }

      queryClient.fetchQuery({
        queryKey: queryKeys.changeIdFromSha(localDir, sha),
        queryFn: async () => {
          console.log(`Fetching change ID for SHA ${sha}`)
          const result = await commands.getChangeIdFromSha(localDir, sha)
          if (result.status === "error") {
            console.error(
              `Error fetching change ID for SHA ${sha}: ${result.error}`,
            )
            setCache((prev) => new Map(prev).set(sha, null))
            return null
          }
          const changeId = result.data ?? null
          setCache((prev) => new Map(prev).set(sha, changeId))
          return changeId
        },
        staleTime: Infinity,
      })

      return undefined
    },
    [localDir, queryClient, cache],
  )

  return (
    <ShaToChangeIdContext.Provider value={{ getChangeId }}>
      {children}
    </ShaToChangeIdContext.Provider>
  )
}

export function useShaToChangeId() {
  const context = useContext(ShaToChangeIdContext)
  if (!context) {
    throw new Error(
      "useShaToChangeId must be used within ShaToChangeIdProvider",
    )
  }
  return context
}
