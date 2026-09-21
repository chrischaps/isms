// The Common Store and the Ledger of Contribution (S2.7): the shelves with
// my entitlement, yesterday's service and my draw record; every citizen's
// public record; and the two commands a norm system needs, taking and
// giving up a position. Every key sits under the society's so a tick on the
// stream refreshes it.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type StoreView = Schemas["StoreView"];
export type StockView = Schemas["StockView"];
export type StoreDayView = Schemas["StoreDayView"];
export type ContributionView = Schemas["ContributionView"];
export type ContributionRow = Schemas["ContributionRow"];
export type PositionView = Schemas["PositionView"];

export const commonsKeys = {
  store: (id: number) => ["society", id, "store"] as const,
  ledger: (id: number) => ["society", id, "ledger"] as const,
};

/** `enabled: false` where there is no Common Store (the route answers 422 there). */
export function useStore(id: number, enabled = true) {
  return useQuery({
    queryKey: commonsKeys.store(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/store", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
    enabled,
  });
}

/** `enabled: false` where labor is not by norm. */
export function useLedger(id: number, enabled = true) {
  return useQuery({
    queryKey: commonsKeys.ledger(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/ledger", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
    enabled,
  });
}

function useSocietyInvalidation(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: keys.society(id) });
  };
}

export function useTakePosition(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (wid: number) =>
      unwrap(await api.POST("/s/{id}/workplaces/{wid}/position", { params: { path: { id, wid } } })),
    onSuccess: done,
  });
}

export function useLeavePosition(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (wid: number) =>
      unwrap(await api.DELETE("/s/{id}/workplaces/{wid}/position", { params: { path: { id, wid } } })),
    onSuccess: done,
  });
}
