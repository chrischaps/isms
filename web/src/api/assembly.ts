// The assembly (S2.5 routes, S2.6 screen): open proposals with their tallies
// and my ballot, those closed this epoch with their outcomes, one proposal,
// the offices with their elections, and the commands: move a proposal, cast
// a ballot, stand, withdraw, approve. Every key sits under the society's so a
// tick on the stream refreshes it.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, unwrap, type Schemas } from "./client";
import { TICK_FALLBACK_MS, keys } from "./hooks";

export type ProposalsView = Schemas["ProposalsView"];
export type ProposalView = Schemas["ProposalView"];
export type ProposalOutcome = Schemas["ProposalOutcome"];
export type TallyView = Schemas["TallyView"];
export type BallotView = Schemas["BallotView"];
export type OfficesView = Schemas["OfficesView"];
export type OfficeView = Schemas["OfficeView"];
export type ElectionView = Schemas["ElectionView"];
export type Ballot = "yes" | "no" | "abstain";

/** The engine's `ProposalKind` on the wire (documented as `Object`; here as it is). */
export type ProposalKind =
  | "resolution"
  | { policy_change: { patch: Record<string, unknown> } }
  | { honor: { citizen: number } }
  | { recall: { office: string; citizen: number } }
  | { election: { office: string } }
  | { admission: { org: number; citizen: number } }
  | { disbursement: { org: number; to: unknown; asset: unknown } };

export const assemblyKeys = {
  proposals: (id: number) => ["society", id, "proposals"] as const,
  proposal: (id: number, pid: number) => ["society", id, "proposals", pid] as const,
  offices: (id: number) => ["society", id, "offices"] as const,
};

export function useProposals(id: number, enabled = true) {
  return useQuery({
    queryKey: assemblyKeys.proposals(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/proposals", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
    enabled,
  });
}

export function useProposal(id: number, pid: number) {
  return useQuery({
    queryKey: assemblyKeys.proposal(id, pid),
    queryFn: async () => unwrap(await api.GET("/s/{id}/proposals/{pid}", { params: { path: { id, pid } } })),
    refetchInterval: TICK_FALLBACK_MS,
  });
}

export function useOffices(id: number, enabled = true) {
  return useQuery({
    queryKey: assemblyKeys.offices(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/offices", { params: { path: { id } } })),
    refetchInterval: TICK_FALLBACK_MS,
    enabled,
  });
}

function useAssemblyInvalidation(id: number) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: assemblyKeys.proposals(id) });
    void qc.invalidateQueries({ queryKey: assemblyKeys.offices(id) });
    void qc.invalidateQueries({ queryKey: keys.home(id) });
  };
}

export function usePropose(id: number) {
  const done = useAssemblyInvalidation(id);
  return useMutation({
    mutationFn: async (body: { title: string; text?: string; kind: ProposalKind }) =>
      unwrap(
        await api.POST("/s/{id}/proposals", {
          params: { path: { id } },
          body: { ...body, kind: body.kind as unknown as Record<string, never> },
        }),
      ),
    onSuccess: done,
  });
}

export function useBallot(id: number) {
  const done = useAssemblyInvalidation(id);
  return useMutation({
    mutationFn: async ({ pid, ballot }: { pid: number; ballot: Ballot }) =>
      unwrap(await api.PUT("/s/{id}/proposals/{pid}/ballot", { params: { path: { id, pid } }, body: { ballot } })),
    onSuccess: done,
  });
}

export function useStand(id: number) {
  const done = useAssemblyInvalidation(id);
  return useMutation({
    mutationFn: async (kind: string) =>
      unwrap(await api.POST("/s/{id}/offices/{kind}/candidacy", { params: { path: { id, kind } } })),
    onSuccess: done,
  });
}

export function useWithdraw(id: number) {
  const done = useAssemblyInvalidation(id);
  return useMutation({
    mutationFn: async (kind: string) =>
      unwrap(await api.DELETE("/s/{id}/offices/{kind}/candidacy", { params: { path: { id, kind } } })),
    onSuccess: done,
  });
}

export function useApprove(id: number) {
  const done = useAssemblyInvalidation(id);
  return useMutation({
    mutationFn: async ({ kind, candidates }: { kind: string; candidates: number[] }) =>
      unwrap(await api.PUT("/s/{id}/offices/{kind}/ballot", { params: { path: { id, kind } }, body: { candidates } })),
    onSuccess: done,
  });
}
