// A mutation that refetches what it may have changed, whether it succeeded or
// not: a refusal (a 409 on a rename, say) leaves the cache stale as surely as
// a success does.

import {
  type QueryKey,
  type UseMutationOptions,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";

/** `useMutation`, then invalidate every key in `keys` once it has settled. */
export function useInvalidatingMutation<TData, TVariables, TContext = unknown>(
  keys: readonly QueryKey[],
  options: UseMutationOptions<TData, Error, TVariables, TContext>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    ...options,
    onSettled: async (...settled) => {
      await options.onSettled?.(...settled);
      await Promise.all(keys.map((queryKey) => queryClient.invalidateQueries({ queryKey })));
    },
  });
}
