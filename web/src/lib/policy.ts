// What a proposal would do, and what a carried one did (S2.6). A
// `PolicyChanged` carries the whole policy after the change, not a diff; the
// proposal's patch names the fields, so the diff is those fields read from
// the policy before (the previous `PolicyChanged` this epoch, when there is
// one) and after.

import type { ProposalKind, TallyView } from "../api/assembly";

export type PolicyValue = string | number | boolean | Record<string, number> | null;

export type DiffRow = { field: string; before: PolicyValue | undefined; after: PolicyValue };

/** `work_norm_hours` -> "work norm hours". */
export function fieldName(field: string): string {
  return field.replaceAll("_", " ");
}

/** One policy value as a person reads it: enums as words, a split as percentages. */
export function policyValue(v: unknown): string {
  if (v === null || v === undefined) return "unset";
  if (typeof v === "string") return v.replaceAll("_", " ");
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : v.toFixed(2);
  if (typeof v === "boolean") return v ? "on" : "off";
  if (typeof v === "object") {
    return Object.entries(v as Record<string, unknown>)
      .map(([k, x]) => `${fieldName(k)} ${typeof x === "number" && x <= 1 ? `${Math.round(x * 100)}%` : String(x)}`)
      .join(", ");
  }
  return String(v);
}

/** The fields a patch sets (a `null` field is not a change). */
export function patchFields(patch: Record<string, unknown>): string[] {
  return Object.keys(patch)
    .filter((k) => patch[k] !== null && patch[k] !== undefined)
    .sort();
}

/** The rows of a carried policy change: each patched field, before and after. */
export function policyDiff(
  patch: Record<string, unknown>,
  before: Record<string, unknown> | null,
  after: Record<string, unknown>,
): DiffRow[] {
  return patchFields(patch).map((field) => ({
    field,
    before: before ? ((before[field] ?? null) as PolicyValue) : undefined,
    after: (after[field] ?? null) as PolicyValue,
  }));
}

/** "work norm hours 6 -> 7", or "work norm hours -> 7" when the earlier value is not on record. */
export function diffText(row: DiffRow): string {
  const to = policyValue(row.after);
  return row.before === undefined ? `${fieldName(row.field)} -> ${to}` : `${fieldName(row.field)} ${policyValue(row.before)} -> ${to}`;
}

/** The kind's tag as a heading: "policy change", "honor". */
export function kindTitle(tag: string): string {
  return tag.replaceAll("_", " ");
}

/** One line on what a proposal would do, with citizens and offices named. */
export function kindSummary(kind: ProposalKind, citizen: (id: number) => string, org: (id: number) => string): string {
  if (kind === "resolution") return "A resolution: recorded as the assembly's minutes, with no other effect.";
  if ("policy_change" in kind) {
    const fields = patchFields(kind.policy_change.patch);
    if (fields.length === 0) return "A policy change that sets nothing.";
    return `Sets ${fields.map((f) => `${fieldName(f)} to ${policyValue(kind.policy_change.patch[f])}`).join("; ")}.`;
  }
  if ("honor" in kind) return `Honors ${citizen(kind.honor.citizen)}: one line on their record, never revoked.`;
  if ("recall" in kind) return `Recalls ${citizen(kind.recall.citizen)} from the office of ${fieldName(kind.recall.office)}.`;
  if ("election" in kind) return `An election for ${fieldName(kind.election.office)}.`;
  if ("admission" in kind) return `Admits ${citizen(kind.admission.citizen)} to ${org(kind.admission.org)}.`;
  if ("disbursement" in kind) return `Moves something out of ${org(kind.disbursement.org)}.`;
  return "";
}

/** Where the quorum line sits on a 0..100 bar of the electorate, and how far the ballots have reached. */
export function quorumBar(t: TallyView): { value: number; threshold: number; met: boolean } {
  if (t.eligible <= 0) return { value: 0, threshold: 100, met: false };
  const threshold = Math.min(100, (t.quorum / t.eligible) * 100);
  const value = Math.min(100, (t.cast / t.eligible) * 100);
  return { value, threshold, met: t.cast >= t.quorum };
}

/** Whether the tally as it stands would carry: quorum reached and more yes than no (Q117). */
export function wouldCarry(t: TallyView): boolean {
  return t.cast >= t.quorum && t.yes > t.no;
}
