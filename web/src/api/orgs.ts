// Queries and mutations for organizations (S1.11): the list with founding
// costs and slot scarcity, one org, and every manager and owner command.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";
import { marketKeys } from "./market";

export type OrgsView = Schemas["OrgsView"];
export type OrgView = Schemas["OrgView"];
export type WorkplaceView = Schemas["WorkplaceView"];
export type WorkerView = Schemas["WorkerView"];
export type FoundOrg = Schemas["FoundOrgRequest"];
export type EmploymentOffer = Schemas["EmploymentOfferRequest"];

export const orgKeys = {
  org: (id: number, oid: number) => ["society", id, "orgs", oid] as const,
};

export function useOrgsView(id: number) {
  return useQuery({
    queryKey: marketKeys.orgs(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/orgs", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useOrg(id: number, oid: number) {
  return useQuery({
    queryKey: orgKeys.org(id, oid),
    queryFn: async () => unwrap(await api.GET("/s/{id}/orgs/{oid}", { params: { path: { id, oid } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

function useInvalidate(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: keys.society(id) });
  };
}

export function useFoundOrg(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: FoundOrg) =>
      unwrap(await api.POST("/s/{id}/orgs", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

export function useOfferEmployment(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: EmploymentOffer) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/offers", { params: { path: { id, oid } }, body })),
    onSuccess: done,
  });
}

export function useMachines(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: { workplace: number; action: "install" | "uninstall"; qty: number }) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/machines", { params: { path: { id, oid } }, body })),
    onSuccess: done,
  });
}

export function useDividend(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (per_share: number) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/dividend", { params: { path: { id, oid } }, body: { per_share } })),
    onSuccess: done,
  });
}

export function useAppoint(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (citizen: number | null) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/manager", { params: { path: { id, oid } }, body: { citizen } })),
    onSuccess: done,
  });
}

export function useIssueShares(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (qty: number) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/shares", { params: { path: { id, oid } }, body: { qty } })),
    onSuccess: done,
  });
}

export function useAddWorkplace(id: number, oid: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: { kind: string; slot?: number | null }) =>
      unwrap(await api.POST("/s/{id}/orgs/{oid}/workplaces", { params: { path: { id, oid } }, body })),
    onSuccess: done,
  });
}

/** List shares (or goods) for sale on the notice board, as yourself or for an org you manage. */
export function useOfferSale(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: { asset: Record<string, unknown>; price: Record<string, unknown>; on_behalf_of?: number | null }) =>
      unwrap(
        await api.POST("/s/{id}/offers/sale", {
          params: { path: { id } },
          body: body as unknown as Schemas["SaleOfferRequest"],
        }),
      ),
    onSuccess: done,
  });
}

export function useCancelOffer(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (oid: number) =>
      unwrap(await api.DELETE("/s/{id}/offers/{oid}", { params: { path: { id, oid } } })),
    onSuccess: done,
  });
}
