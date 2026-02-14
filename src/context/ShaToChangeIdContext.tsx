import { useQueryClient } from "@tanstack/react-query"
import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useState,
} from "react"

import { commands } from "@/bindings"
import { queryKeys } from "@/lib/queryKeys"

type ShaToChangeIdContextValue = {
  getChangeId: (
    sha: string,
    localDir: string | null,
  ) => string | null | undefined
}

const ShaToChangeIdContext = createContext<ShaToChangeIdContextValue | null>(
  null,
)

type ShaToChangeIdProviderProps = {
  children: ReactNode
}

export function ShaToChangeIdProvider({
  children,
}: ShaToChangeIdProviderProps) {
  const queryClient = useQueryClient()

  // Cache keyed by "localDir:sha"
  const [cache, setCache] = useState<Map<string, string | null>>(new Map())

  const getChangeId = useCallback(
    (sha: string, localDir: string | null) => {
      if (!localDir) return undefined

      const cacheKey = `${localDir}:${sha}`

      if (cache.has(cacheKey)) {
        return cache.get(cacheKey)!
      }

      queryClient.fetchQuery({
        queryKey: queryKeys.changeIdFromSha(localDir, sha),
        queryFn: async () => {
          const result = await commands.getChangeIdFromSha(localDir, sha)
          if (result.status === "error") {
            setCache((prev) => new Map(prev).set(cacheKey, null))
            return null
          }
          const changeId = result.data ?? null
          setCache((prev) => new Map(prev).set(cacheKey, changeId))
          return changeId
        },
        staleTime: Infinity,
      })

      return undefined
    },
    [cache, queryClient],
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
