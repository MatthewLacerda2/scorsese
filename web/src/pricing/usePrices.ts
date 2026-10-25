// What the pricing page shows, read from the server (`GET /api/prices`,
// #1538): the examples, the free allowance and the recharge cards at the day's
// rate, all from the tables that charge, so the page cannot disagree with what
// is charged.

import { useQuery } from "@tanstack/react-query";
import { api } from "@/api";
import { queryKeys } from "@/app/queryKeys";

/** How long the figures are kept before they are asked for again: the rate moves. */
const FRESH_MS = 60_000;

export function usePrices() {
  return useQuery({
    queryKey: queryKeys.prices,
    queryFn: api.prices,
    staleTime: FRESH_MS,
    refetchInterval: FRESH_MS,
  });
}
