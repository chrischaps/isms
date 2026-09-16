// Queries and mutations for life inside a society (S1.8): the plan, payslips,
// the job board, joining, taking a job, setting labor.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type PayslipsView = Schemas["PayslipsView"];
export type PlanView = Schemas["PlanView"];
export type Joined = Schemas["Joined"];
export type LaborView = Schemas["LaborView"];
export type AllocationView = Schemas["AllocationView"];
export type Effort = "low" | "normal" | "high";
export type Allocation = { workplace: number; hours: number; effort: Effort };

export const societyKeys = {
  plan: (id: number) => ["society", id, "plan"] as const,
  payslips: (id: number) => ["society", id, "payslips"] as const,
};

export function usePlan(id: number) {
  return useQuery({
    queryKey: societyKeys.plan(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/plan", { params: { path: { id } } })),
  });
}

export function usePayslips(id: number) {
  return useQuery({
    queryKey: societyKeys.payslips(id),
    queryFn: async () =>
      unwrap(await api.GET("/s/{id}/payslips", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

function useSocietyInvalidation(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: keys.society(id) });
    void qc.invalidateQueries({ queryKey: keys.me });
  };
}

export function useJoin(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (handle: string) =>
      unwrap(
        await api.POST("/societies/{id}/join", {
          params: { path: { id } },
          body: { handle },
        }),
      ),
    onSuccess: done,
  });
}

export function useAcceptOffer(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (oid: number) =>
      unwrap(
        await api.POST("/s/{id}/offers/{oid}/accept", {
          params: { path: { id, oid } },
          body: {},
        }),
      ),
    onSuccess: done,
  });
}

export function useSetLabor(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (allocations: Allocation[]) =>
      unwrap(
        await api.PUT("/s/{id}/labor", {
          params: { path: { id } },
          body: { allocations },
        }),
      ),
    onSuccess: done,
  });
}

export function useSetPlan(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (plan: Record<string, unknown>) =>
      unwrap(
        await api.PUT("/s/{id}/plan", {
          params: { path: { id } },
          body: { plan: plan as Record<string, never> },
        }),
      ),
    onSuccess: done,
  });
}
