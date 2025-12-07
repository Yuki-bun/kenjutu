import { QueryFunction, QueryKey, useQuery, UseQueryOptions, UseQueryResult, useMutation, UseMutationOptions, MutationFunction, UseMutationResult } from "@tanstack/react-query";
import { Result } from "../bindings";


class ClientError extends Error {
  constructor(error: unknown) {
    super(JSON.stringify(error))
  }
}


interface RpcQueryOptions<TData, TError, TQueryKey extends QueryKey> extends Omit<UseQueryOptions<TData, TError, TData, TQueryKey>, 'queryFn' | 'initialData' | 'throwOnError'> {
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
      let result: Result<TData, TError>
      try {
        result = await options.queryFn(args);
      } catch (error) {
        throw new ClientError(error)
      }
      if (result.status === "error") {
        throw result.error
      }
      return result.data;
    },
    throwOnError: (error) => {
      return error instanceof ClientError
    }
  });
}


interface RpcMutaionOptions<TData, TError, TVariables, TOnMutateResult> extends Omit<UseMutationOptions<TData, TError, TVariables, TOnMutateResult>, 'mutationFn' | 'throwOnError'> {
  mutationFn: MutationFunction<Result<TData, TError>, TVariables>
}

export function useRpcMutation<
  TData,
  TError,
  TVariables,
  TOnMutateResult
>(
  options: RpcMutaionOptions<TData, TError, TVariables, TOnMutateResult>
): UseMutationResult<TData, TError, TVariables, TOnMutateResult> {
  return useMutation({
    ...options,
    mutationFn: async (variables, context) => {
      let result: Result<TData, TError>
      try {
        result = await options.mutationFn(variables, context)
      } catch (error) {
        throw new ClientError(error)
      }

      if (result.status == 'error') {
        throw result.error
      }

      return result.data
    }
  })
}
