// Society-wide reads and the talk channels (S1.13): stats, the Chronicle by
// cycle, the scoreboard, citizens, the householder script, one event with
// its Explains, messages on a channel or a DM, and the account's profile.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type StatsView = Schemas["StatsView"];
export type ChronicleView = Schemas["ChronicleView"];
export type HeadlineView = Schemas["HeadlineView"];
export type ScoreboardView = Schemas["ScoreboardView"];
export type CitizensView = Schemas["CitizensView"];
export type CitizenPublic = Schemas["CitizenPublic"];
export type ExplainView = Schemas["ExplainView"];
export type MessagesView = Schemas["MessagesView"];
export type MessageView = Schemas["MessageView"];
export type Me = Schemas["Me"];
export type ArchivesView = Schemas["ArchivesView"];
export type ArchiveView = Schemas["ArchiveView"];
export type ApiKeyCreated = Schemas["ApiKeyCreated"];

export const civicKeys = {
  stats: (id: number) => ["society", id, "stats"] as const,
  chronicle: (id: number, cycle: number | null) => ["society", id, "chronicle", cycle] as const,
  scoreboard: (id: number) => ["society", id, "scoreboard"] as const,
  citizens: (id: number) => ["society", id, "citizens"] as const,
  householders: (id: number) => ["society", id, "householders"] as const,
  event: (id: number, seq: number) => ["society", id, "event", seq] as const,
  channel: (id: number, channel: string) => ["society", id, "talk", channel] as const,
  publicSocieties: ["public", "societies"] as const,
  publicStats: (id: number) => ["public", id, "stats"] as const,
  publicChronicle: (id: number, cycle: number | null) => ["public", id, "chronicle", cycle] as const,
  archives: (id: number) => ["society", id, "archives"] as const,
  publicArchives: (id: number) => ["public", id, "archives"] as const,
};

export function useStats(id: number) {
  return useQuery({
    queryKey: civicKeys.stats(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/stats", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useChronicle(id: number, cycle: number | null) {
  return useQuery({
    queryKey: civicKeys.chronicle(id, cycle),
    queryFn: async () =>
      unwrap(await api.GET("/s/{id}/chronicle", { params: { path: { id }, query: { cycle } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useScoreboard(id: number) {
  return useQuery({
    queryKey: civicKeys.scoreboard(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/scoreboard", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useCitizens(id: number) {
  return useQuery({
    queryKey: civicKeys.citizens(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/citizens", { params: { path: { id } } })),
  });
}

export function useHouseholders(id: number) {
  return useQuery({
    queryKey: civicKeys.householders(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/householders", { params: { path: { id } } })),
    staleTime: Infinity,
  });
}

export function useEvent(id: number, seq: number) {
  return useQuery({
    queryKey: civicKeys.event(id, seq),
    queryFn: async () => unwrap(await api.GET("/s/{id}/explain/{seq}", { params: { path: { id, seq } } })),
    staleTime: Infinity,
  });
}

/** `square`, `org:<id>`, or `dm:<citizen>` (the other party). */
export function useChannel(id: number, channel: string) {
  const dm = channel.startsWith("dm:") ? Number(channel.slice(3)) : null;
  return useQuery({
    queryKey: civicKeys.channel(id, channel),
    queryFn: async () =>
      dm !== null
        ? unwrap(await api.GET("/s/{id}/dm/{citizen}", { params: { path: { id, citizen: dm } } }))
        : unwrap(await api.GET("/s/{id}/channels/{channel}/messages", { params: { path: { id, channel } } })),
    refetchInterval: 5_000,
  });
}

export function usePost(id: number, channel: string) {
  const qc = useQueryClient();
  const dm = channel.startsWith("dm:") ? Number(channel.slice(3)) : null;
  return useMutation({
    mutationFn: async (body: string) =>
      dm !== null
        ? unwrap(await api.POST("/s/{id}/dm/{citizen}", { params: { path: { id, citizen: dm } }, body: { body } }))
        : unwrap(
            await api.POST("/s/{id}/channels/{channel}/messages", { params: { path: { id, channel } }, body: { body } }),
          ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: civicKeys.channel(id, channel) });
    },
  });
}

export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: { label: string; society_id: number }) =>
      unwrap(await api.POST("/me/api-keys", { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.me });
    },
  });
}

export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: number) => unwrap(await api.DELETE("/me/api-keys/{id}", { params: { path: { id } } })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.me });
    },
  });
}

export function useUpdateMe() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: { biography: string }) => unwrap(await api.PATCH("/me", { body })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: keys.me });
    },
  });
}

// -- spectator routes: no login ------------------------------------------------

export function usePublicSocieties() {
  return useQuery({
    queryKey: civicKeys.publicSocieties,
    queryFn: async () => unwrap(await api.GET("/public/societies")).societies,
  });
}

export function usePublicStats(id: number) {
  return useQuery({
    queryKey: civicKeys.publicStats(id),
    queryFn: async () => unwrap(await api.GET("/public/s/{id}/stats", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

/** Every finished epoch, with the caller's own closing statement marked (S1.15). */
export function useArchives(id: number) {
  return useQuery({
    queryKey: civicKeys.archives(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/archives", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function usePublicArchives(id: number) {
  return useQuery({
    queryKey: civicKeys.publicArchives(id),
    queryFn: async () => unwrap(await api.GET("/public/s/{id}/archives", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

/** One closing statement per citizen, replaced on every write, until the window closes. */
export function useClosingStatement(id: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (text: string) =>
      unwrap(await api.PUT("/s/{id}/closing-statement", { params: { path: { id } }, body: { text } })),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: civicKeys.archives(id) });
      void qc.invalidateQueries({ queryKey: civicKeys.publicArchives(id) });
    },
  });
}

export function usePublicChronicle(id: number, cycle: number | null) {
  return useQuery({
    queryKey: civicKeys.publicChronicle(id, cycle),
    queryFn: async () =>
      unwrap(await api.GET("/public/s/{id}/chronicle", { params: { path: { id }, query: { cycle } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}
