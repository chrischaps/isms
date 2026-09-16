// The operator's routes (S1.13c): list societies with their clocks, hold and
// release a clock, step one tick, change the tick length, end an epoch.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { keys } from "./hooks";

export type AdminSocietyView = Schemas["AdminSocietyView"];

export const adminKeys = { societies: ["admin", "societies"] as const };

export function useAdminSocieties(enabled: boolean) {
  return useQuery({
    queryKey: adminKeys.societies,
    queryFn: async () => unwrap(await api.GET("/admin/societies")).societies,
    refetchInterval: 2_000,
    enabled,
  });
}

type Act = "pause" | "resume" | "step" | "end-epoch";

export function useAdminAct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, act }: { id: number; act: Act }) => {
      const params = { params: { path: { id } } };
      switch (act) {
        case "pause":
          return unwrap(await api.POST("/admin/s/{id}/pause", params));
        case "resume":
          return unwrap(await api.POST("/admin/s/{id}/resume", params));
        case "step":
          return unwrap(await api.POST("/admin/s/{id}/step", params));
        case "end-epoch":
          return unwrap(await api.POST("/admin/s/{id}/end-epoch", params));
      }
    },
    onSuccess: (_r, { id }) => {
      void qc.invalidateQueries({ queryKey: adminKeys.societies });
      void qc.invalidateQueries({ queryKey: keys.society(id) });
      void qc.invalidateQueries({ queryKey: keys.societies });
    },
  });
}

export function useAdminTickSeconds() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, tick_seconds }: { id: number; tick_seconds: number }) =>
      unwrap(await api.POST("/admin/s/{id}/tick-seconds", { params: { path: { id } }, body: { tick_seconds } })),
    onSuccess: (_r, { id }) => {
      void qc.invalidateQueries({ queryKey: adminKeys.societies });
      void qc.invalidateQueries({ queryKey: keys.society(id) });
    },
  });
}
