// Thin TanStack Query hooks over the client, plus the two hooks the whole
// client is structured around (TDD 11): capabilities decide what mounts,
// the lexicon supplies every label. A screen never checks the preset name.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { api, unwrap, type StreamFrame } from "./client";

export const keys = {
  me: ["me"] as const,
  societies: ["societies"] as const,
  society: (id: number) => ["society", id] as const,
  capabilities: (id: number) => ["society", id, "capabilities"] as const,
  lexicon: (id: number) => ["society", id, "lexicon"] as const,
  welcome: (id: number) => ["society", id, "welcome"] as const,
  home: (id: number) => ["society", id, "home"] as const,
  board: (id: number) => ["society", id, "board"] as const,
};

export function useMe() {
  return useQuery({
    queryKey: keys.me,
    queryFn: async () => unwrap(await api.GET("/me")),
    retry: false,
  });
}

export function useSocieties() {
  return useQuery({
    queryKey: keys.societies,
    queryFn: async () => unwrap(await api.GET("/societies")).societies,
  });
}

export function useSociety(id: number) {
  return useQuery({
    queryKey: keys.society(id),
    queryFn: async () =>
      unwrap(await api.GET("/societies/{id}", { params: { path: { id } } })),
  });
}

/** What exists in this society; nav items and widgets mount on it. */
export function useCapabilities(id: number) {
  return useQuery({
    queryKey: keys.capabilities(id),
    queryFn: async () =>
      unwrap(
        await api.GET("/societies/{id}/capabilities", {
          params: { path: { id } },
        }),
      ),
    staleTime: Infinity,
  });
}

/** `t("compensation")` -> "Payslip" here, "Draw record" elsewhere. */
export function useLexicon(id: number) {
  const q = useQuery({
    queryKey: keys.lexicon(id),
    queryFn: async () =>
      unwrap(
        await api.GET("/societies/{id}/lexicon", { params: { path: { id } } }),
      ),
    staleTime: Infinity,
  });
  const entries = q.data?.entries ?? {};
  const t = (key: string): string => entries[key] ?? key;
  return { ...q, t };
}

export function useWelcome(id: number) {
  return useQuery({
    queryKey: keys.welcome(id),
    queryFn: async () =>
      unwrap(
        await api.GET("/societies/{id}/welcome", { params: { path: { id } } }),
      ),
  });
}

export function useHome(id: number) {
  return useQuery({
    queryKey: keys.home(id),
    queryFn: async () =>
      unwrap(await api.GET("/s/{id}/home", { params: { path: { id } } })),
  });
}

export function useBoard(id: number) {
  return useQuery({
    queryKey: keys.board(id),
    queryFn: async () =>
      unwrap(
        await api.GET("/s/{id}/notice-board", { params: { path: { id } } }),
      ),
  });
}

/** Subscribe to the society's stream and invalidate its queries on every
 *  frame: screens update when a tick lands without polling (TDD 11). */
export function useStream(id: number | undefined, onFrame?: (f: StreamFrame) => void) {
  const qc = useQueryClient();
  useEffect(() => {
    if (id === undefined) return;
    const proto = location.protocol === "https:" ? "wss" : "ws";
    const base = import.meta.env.VITE_API_BASE
      ? import.meta.env.VITE_API_BASE.replace(/^http/, "ws")
      : `${proto}://${location.host}`;
    const ws = new WebSocket(`${base}/s/${id}/stream`);
    ws.onmessage = (m) => {
      try {
        const frame = JSON.parse(String(m.data)) as StreamFrame;
        onFrame?.(frame);
        if (frame.events.length > 0) {
          void qc.invalidateQueries({ queryKey: keys.society(id) });
        }
      } catch {
        // a malformed frame is ignored; the next one will do
      }
    };
    return () => ws.close();
  }, [id, qc, onFrame]);
}
