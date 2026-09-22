// What kind of society the cohort is playing: the constitution's capabilities
// as `/societies/{id}/capabilities` states them, plus the preset's name. Read
// once per run, outside any player's turn, and handed to both brains: the
// scripted strategies choose their moves by it, the fuzzer its probes, the
// LLM brain the words of its rules. A brain built without one plays Freeport.

import { type Client, unwrap } from "./api/client.ts";

export type SocietyFacts = {
  preset: string;
  display: string;
  money: boolean;
  /** `none`, `direct`, `representative`, ... (the constitution's `governance`). */
  governance: string;
  /** `free`, `norm`, `assigned`. */
  labor: string;
  common_store: boolean;
  /** Office kinds the constitution seats. */
  offices: string[];
  /** Proposal kinds the assembly may open. */
  proposal_kinds: string[];
  /** `anyone` or `office_holders`. */
  proposers: string;
};

export const FREEPORT: SocietyFacts = {
  preset: "freeport",
  display: "Freeport",
  money: true,
  governance: "none",
  labor: "free",
  common_store: false,
  offices: [],
  proposal_kinds: [],
  proposers: "anyone",
};

/** True where an assembly sits: proposals, ballots, offices. */
export function hasAssembly(f: SocietyFacts): boolean {
  return f.governance !== "none" && f.proposal_kinds.length > 0;
}

export async function readFacts(client: Client, sid: number): Promise<SocietyFacts> {
  const caps = unwrap(await client.GET("/societies/{id}/capabilities", { params: { path: { id: sid } } }));
  const society = unwrap(await client.GET("/societies/{id}", { params: { path: { id: sid } } }));
  return {
    preset: society.preset,
    display: society.display,
    money: caps.money,
    governance: caps.governance,
    labor: caps.labor,
    common_store: caps.common_store,
    offices: (caps.offices as { kind: string }[]).map((o) => o.kind),
    proposal_kinds: caps.proposal_kinds ?? [],
    proposers: caps.proposers ?? "anyone",
  };
}
