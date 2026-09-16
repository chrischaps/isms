// Queries and mutations for contracts, the notice board's typed ads,
// housing and transfers (S1.12).

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type ContractView = Schemas["ContractView"];
export type CreditOffer = Schemas["CreditOfferRequest"];
export type LeaseOffer = Schemas["LeaseOfferRequest"];
export type Wanted = Schemas["WantedRequest"];
export type Transfer = Schemas["TransferRequest"];

export const contractKeys = {
  contracts: (id: number) => ["society", id, "contracts"] as const,
};

export function useContracts(id: number) {
  return useQuery({
    queryKey: contractKeys.contracts(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/contracts", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

function useInvalidate(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: keys.society(id) });
  };
}

export function useOfferCredit(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: CreditOffer) =>
      unwrap(await api.POST("/s/{id}/offers/credit", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

export function useOfferLease(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: LeaseOffer) =>
      unwrap(await api.POST("/s/{id}/offers/lease", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

export function usePostWanted(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: Wanted) =>
      unwrap(await api.POST("/s/{id}/offers/wanted", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

export function useTransfer(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (body: Transfer) =>
      unwrap(await api.POST("/s/{id}/transfers", { params: { path: { id } }, body })),
    onSuccess: done,
  });
}

/** End an employment contract or a lease; the engine applies the notice rules. */
export function useTerminate(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (cid: number) =>
      unwrap(await api.POST("/s/{id}/contracts/{cid}/terminate", { params: { path: { id, cid } }, body: {} })),
    onSuccess: done,
  });
}

export function useMoveOut(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (did: number) =>
      unwrap(await api.POST("/s/{id}/dwellings/{did}/move-out", { params: { path: { id, did } } })),
    onSuccess: done,
  });
}

export function useMoveIn(id: number) {
  const done = useInvalidate(id);
  return useMutation({
    mutationFn: async (did: number) =>
      unwrap(await api.POST("/s/{id}/dwellings/{did}/move-in", { params: { path: { id, did } } })),
    onSuccess: done,
  });
}
