import { QueryFunction, QueryKey, useQuery, UseQueryOptions, UseQueryResult } from "@tanstack/react-query";
import { Result } from "../bindings";


class ClientError extends Error {
  constructor(error: unknown) {
    super(JSON.stringify(error))
  }
}


interface RpcQueryOptions<TData, TError, TQueryKey extends QueryKey> extends Omit<UseQueryOptions<TData, TError, TData, TQueryKey>, 'queryFn' | 'initialData'> {
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
      try {
        const result = await options.queryFn(args);
        if (result.status === "error") {
          throw result.error
        }

        return result.data;
      } catch (error) {
        throw new ClientError(error)
      }
    },
    throwOnError: (error) => {
      return error instanceof ClientError
    }
  });
}
