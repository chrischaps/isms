// The Coordinator workspace (S2.8): the Plan as published, with every
// workplace's target beside what it made and the land it is published over;
// and the coordinator's three commands, publish the Plan, open a workplace,
// close one. The rationing proposal goes through the assembly's `usePropose`.
// Every key sits under the society's so a tick on the stream refreshes it.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type PublishedPlanView = Schemas["PublishedPlanView"];
export type PlanTargetView = Schemas["PlanTargetView"];
export type SlotView = Schemas["SlotView"];

export const coordinatorKeys = {
  plan: (id: number) => ["society", id, "plan", "published"] as const,
};

/** `enabled: false` where nobody publishes a Plan (the route answers 422 there). */
export function usePublishedPlan(id: number, enabled = true) {
  return useQuery({
    queryKey: coordinatorKeys.plan(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/plan/published", { params: { path: { id } } })),
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

export function usePublishPlan(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (targets: Record<string, number>) =>
      unwrap(await api.PUT("/s/{id}/offices/coordinator/plan", { params: { path: { id } }, body: { targets } })),
    onSuccess: done,
  });
}

export function useOpenWorkplace(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (body: { kind: string; slot?: number | null }) =>
      unwrap(await api.POST("/s/{id}/workplaces", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

export function useCloseWorkplace(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (wid: number) => unwrap(await api.DELETE("/s/{id}/workplaces/{wid}", { params: { path: { id, wid } } })),
    onSuccess: done,
  });
}
