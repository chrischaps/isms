// Queries and mutations for the market (S1.10): the instrument list, one
// book with its tape, price history, placing and cancelling orders.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type BooksView = Schemas["BooksView"];
export type BookSummary = Schemas["BookSummary"];
export type BookView = Schemas["BookView"];
export type OrderView = Schemas["OrderView"];
export type PricesView = Schemas["PricesView"];
export type OrgView = Schemas["OrgView"];
export type PlaceOrder = Schemas["PlaceOrderRequest"];

export const marketKeys = {
  books: (id: number) => ["society", id, "books"] as const,
  book: (id: number, instrument: string) => ["society", id, "books", instrument] as const,
  prices: (id: number, window: number) => ["society", id, "prices", window] as const,
  orgs: (id: number) => ["society", id, "orgs"] as const,
};

export function useBooks(id: number) {
  return useQuery({
    queryKey: marketKeys.books(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/books", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useBook(id: number, instrument: string) {
  return useQuery({
    queryKey: marketKeys.book(id, instrument),
    queryFn: async () =>
      unwrap(await api.GET("/s/{id}/books/{instrument}", { params: { path: { id, instrument } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

/** Per-tick VWAP per instrument over the last `window` ticks. */
export function usePrices(id: number, window: number) {
  return useQuery({
    queryKey: marketKeys.prices(id, window),
    queryFn: async () =>
      unwrap(await api.GET("/s/{id}/prices", { params: { path: { id }, query: { window } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useOrgs(id: number) {
  return useQuery({
    queryKey: marketKeys.orgs(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/orgs", { params: { path: { id } } })).orgs,
  });
}

function useSocietyInvalidation(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: keys.society(id) });
  };
}

export function usePlaceOrder(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (order: PlaceOrder) =>
      unwrap(await api.POST("/s/{id}/orders", { params: { path: { id } }, body: order })),
    onSuccess: done,
  });
}

export function useCancelOrder(id: number) {
  const done = useSocietyInvalidation(id);
  return useMutation({
    mutationFn: async (oid: number) =>
      unwrap(await api.DELETE("/s/{id}/orders/{oid}", { params: { path: { id, oid } } })),
    onSuccess: done,
  });
}
