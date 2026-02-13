import { useQuery } from "@tanstack/react-query"

import { queryKeys } from "@/lib/queryKeys"
import { getLocalRepoDirs } from "@/lib/repos"

export function useLocalRepos() {
  return useQuery({
    queryKey: queryKeys.localRepos(),
    queryFn: getLocalRepoDirs,
  })
}
