// The two Home facts GDD 15 gives a Commune and Freeport lacks (S2.11):
// tonight's ballots, and your line of the Ledger of Contribution. Pure text
// from the wire's counts, so the screen stays a layout and this is testable.

export type OpenBallot = { open: boolean; my_ballot?: string | null };

/** "2 proposals close tonight; 1 waits for your ballot" — or why there is nothing to cast. */
export function ballotsText(proposals: OpenBallot[]): { value: string; gloss?: string; owed: number } {
  const open = proposals.filter((p) => p.open);
  const owed = open.filter((p) => !p.my_ballot).length;
  if (open.length === 0) return { value: "nothing before the assembly", owed: 0 };
  const n = `${open.length} ${open.length === 1 ? "proposal" : "proposals"} open`;
  if (owed === 0) return { value: n, gloss: "you have cast on every one", owed };
  return { value: n, gloss: `${owed} ${owed === 1 ? "waits" : "wait"} for your ballot`, owed };
}

/** "4 of 6 hours today" against the norm, with the standing as the gloss. */
export function lineText(me: { hours_today: number; norm_met_today: boolean; days: number; norm_met_days: number } | undefined, norm: number | null): { value: string; gloss?: string } {
  if (!me) return { value: "not on the record yet" };
  const h = Number.isInteger(me.hours_today) ? String(me.hours_today) : me.hours_today.toFixed(1);
  const value = norm == null ? `${h} h today` : `${h} of ${norm} hours today`;
  const gloss = norm == null ? undefined : me.norm_met_today ? "the norm is met" : me.days > 0 ? `norm met ${me.norm_met_days} of ${me.days} days so far` : "the record opens tonight";
  return { value, gloss };
}
