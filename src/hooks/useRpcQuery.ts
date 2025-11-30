import { QueryFunction, QueryKey, useQuery, UseQueryOptions, UseQueryResult } from "@tanstack/react-query";
import { Result } from "../bindings";

interface RpcQueryOptions<TData, TError, TQueryKey extends QueryKey> extends Omit<UseQueryOptions<TData, TError, TQueryKey>, 'queryFn' | 'initialData'> {
  queryFn: QueryFunction<Result<TData, TError>, TQueryKey>
}

export function useFailableQuery<
  TData,
  TError,
  TQueryKey extends QueryKey
>(
  options: RpcQueryOptions<TData, TError, TQueryKey>
): UseQueryResult<TData, TError> {
  return useQuery({
    ...options,
    queryFn: async (args) => {
      const result = await options.queryFn(args);

      if (result.status === "error") {
        throw result.error;
      }

      return result.data;
    },
  });
}
