// Fold a run into one markdown report: the headline numbers, every rejection
// grouped by code and then by its text with ids blanked, the fuzzer's defects,
// what players did not understand grouped by the endpoint they name, one arc
// per player drawn from the journal, and what the harness itself got wrong.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { readJournal, type Record as JournalRecord, type TurnRecord } from "../journal/journal.ts";
import type { Summary } from "../cohort/cohort.ts";

const ENDPOINT_WORDS: [RegExp, string][] = [
  [/\b(job|employ|hire|offer|notice|board|contract|notice_cycles|max_hours)\b/i, "offers and contracts"],
  [/\b(order|book|bid|ask|instrument|escrow|price|tape|market)\b/i, "market"],
  [/\b(labor|labour|hours|effort|budget|shift|allocation)\b/i, "labor"],
  [/\b(plan|standing|keep_food|keep_balance)\b/i, "standing plan"],
  [/\b(org|firm|found|workplace|machine|dividend|share|treasury|manager)\b/i, "orgs"],
  [/\b(credit|loan|installment|collateral|default|borrow|lend)\b/i, "credit"],
  [/\b(lease|rent|dwelling|house|housing|move)\b/i, "housing"],
  [/\b(food|wares|comfort|shelter|hardship|pantry|needs?)\b/i, "needs"],
  [/\b(chronicle|headline|stats|scoreboard|explain)\b/i, "society"],
  [/\b(proposal|propose|ballot|vote|assembly|quorum|office|coordinator|recall|honou?r|floor|store|ledger|norm|ration|position|entitlement|draw)\b/i, "governance"],
];

/** The act tools that touch the assembly, the offices, the coordinator's powers and the norm (S2.10). */
export const GOVERNANCE_TOOLS = ["propose", "vote", "approve", "stand", "withdraw_candidacy", "publish_plan", "open_workplace", "close_workplace", "post_floor", "take_position", "leave_position"] as const;

/** One paragraph on what the cohort did with the governance tools: counts accepted / refused per tool, the motions moved and who moved them. */
export function governanceParagraph(turns: TurnRecord[]): string {
  const counts = new Map<string, { ok: number; refused: number; players: Set<string> }>();
  const moved: string[] = [];
  for (const t of turns) {
    for (const c of t.calls) {
      if (!(GOVERNANCE_TOOLS as readonly string[]).includes(c.tool)) continue;
      const row = counts.get(c.tool) ?? { ok: 0, refused: 0, players: new Set<string>() };
      if (c.ok) row.ok += 1;
      else row.refused += 1;
      row.players.add(t.player);
      counts.set(c.tool, row);
      if (c.tool === "propose" && c.ok) {
        const i = c.input as { title?: string; kind?: unknown };
        const kind = typeof i.kind === "string" ? i.kind : Object.keys((i.kind as Record<string, unknown>) ?? {})[0] ?? "?";
        moved.push(`"${i.title ?? "?"}" (${kind}, ${t.player}, day ${t.cycle})`);
      }
    }
  }
  if (counts.size === 0) return "No governance tool was used: there is no assembly in this society, or nobody reached for it.";
  const parts = [...counts].map(([tool, r]) => `${tool} ${r.ok} accepted${r.refused ? `, ${r.refused} refused` : ""} (${[...r.players].join(", ")})`);
  const motions = moved.length ? ` Motions moved: ${moved.join("; ")}.` : " No motion was moved.";
  return `Governance tools: ${parts.join("; ")}.${motions}`;
}

/** The Jev players measured by their decisions (SJ.1): how often they held, how sure they were, how often the floors overruled them, how often a slot flipped. */
export function jevSection(turns: TurnRecord[]): string[] {
  const jev = turns.filter((t) => t.brain === "jev");
  if (jev.length === 0) return [];
  const out = ["## Jev decisions", "", "| player | turns | held | mean confidence | dropped by floor | flip-flops | refused | errors | ms/turn | $/turn |", "|---|---|---|---|---|---|---|---|---|---|"];
  const byPlayer = new Map<string, TurnRecord[]>();
  for (const t of jev) byPlayer.set(t.player, [...(byPlayer.get(t.player) ?? []), t]);
  for (const [player, ts] of byPlayer) {
    const decided = ts.filter((t) => t.ended_by !== "error");
    const decisions = decided.flatMap((t) => t.decisions ?? []);
    const held = decided.filter((t) => !(t.decisions ?? []).some((d) => d.acted)).length;
    const dropped = decisions.filter((d) => !d.none && !d.acted).length;
    const mean = decisions.length ? decisions.reduce((n, d) => n + d.confidence, 0) / decisions.length : 0;
    // A slot whose option goes A, B, A over three consecutive turns.
    const bySlot = new Map<string, string[]>();
    for (const t of decided) for (const d of t.decisions ?? []) bySlot.set(d.slot, [...(bySlot.get(d.slot) ?? []), d.option]);
    let flips = 0;
    for (const seq of bySlot.values()) for (let i = 2; i < seq.length; i++) if (seq[i] === seq[i - 2] && seq[i] !== seq[i - 1]) flips += 1;
    const refused = ts.reduce((n, t) => n + t.rejections.length, 0);
    const errors = ts.length - decided.length;
    const ms = ts.length ? Math.round(ts.reduce((n, t) => n + t.ms, 0) / ts.length) : 0;
    const usd = ts.length ? ts.reduce((n, t) => n + t.usage.usd, 0) / ts.length : 0;
    out.push(
      `| ${player} | ${ts.length} | ${held} (${decided.length ? Math.round((held / decided.length) * 100) : 0}%) | ${mean.toFixed(2)} | ${dropped} of ${decisions.length} | ${flips} | ${refused} | ${errors} | ${ms} | ${usd.toFixed(5)} |`,
    );
  }
  // What each slot chose, over the whole cohort.
  const tally = new Map<string, Map<string, number>>();
  for (const t of jev) for (const d of t.decisions ?? []) {
    const m = tally.get(d.slot) ?? new Map<string, number>();
    m.set(d.option, (m.get(d.option) ?? 0) + 1);
    tally.set(d.slot, m);
  }
  if (tally.size) {
    out.push("", "| slot | options chosen |", "|---|---|");
    for (const [slot, m] of [...tally].sort((a, b) => a[0].localeCompare(b[0]))) {
      out.push(`| ${slot} | ${[...m].sort((a, b) => b[1] - a[1]).map(([o, n]) => `${o} ${n}`).join(", ")} |`);
    }
  }
  out.push("", "Held: a turn in which no chosen option ran. Dropped by floor: a choice to act that fell under `jev.min_confidence` or, for an irreversible one, `jev.irreversible_confidence`. Flip-flops: a slot answered A, B, A on three consecutive turns.", "");
  return out;
}

export function endpointOf(text: string): string {
  for (const [re, name] of ENDPOINT_WORDS) if (re.test(text)) return name;
  return "general";
}

/** `c41 already works at w19` -> `c# already works at w#`, so one bug is one row. */
export function normalise(detail: string): string {
  return detail
    .replace(/\b([a-z]{1,2})\d+\b/g, "$1#")
    .replace(/\d+(\.\d+)?/g, "#")
    .trim();
}

type Rejection = { code: string; text: string; count: number; players: Set<string>; tools: Set<string>; example: string };

export type Folded = {
  run: string;
  summary: Summary | null;
  stats: Record<string, unknown> | null;
  /** S1.15: what the epoch-end sequence left behind, when the run waited for it. */
  rollover: Record<string, unknown> | null;
  players: Map<string, JournalRecord[]>;
  turns: TurnRecord[];
};

export function fold(runDir: string, run: string): Folded {
  const players = new Map<string, JournalRecord[]>();
  for (const f of readdirSync(runDir)) {
    if (!f.endsWith(".journal.jsonl")) continue;
    players.set(f.replace(/\.journal\.jsonl$/, ""), readJournal(join(runDir, f)));
  }
  const summaryFile = join(runDir, "summary.json");
  const statsFile = join(runDir, "stats.json");
  const rolloverFile = join(runDir, "rollover.json");
  const turns = [...players.values()].flat().filter((r): r is TurnRecord => r.kind === "turn");
  return {
    run,
    summary: existsSync(summaryFile) ? (JSON.parse(readFileSync(summaryFile, "utf8")) as Summary) : null,
    stats: existsSync(statsFile) ? (JSON.parse(readFileSync(statsFile, "utf8")) as Record<string, unknown> | null) : null,
    rollover: existsSync(rolloverFile) ? (JSON.parse(readFileSync(rolloverFile, "utf8")) as Record<string, unknown>) : null,
    players,
    turns,
  };
}

const cr = (cents: number) => (cents / 100).toFixed(2);

function arc(name: string, records: JournalRecord[]): string {
  const turns = records.filter((r): r is TurnRecord => r.kind === "turn");
  if (turns.length === 0) return `**${name}** took no turns.`;
  const first = turns[0]!;
  const last = turns[turns.length - 1]!;
  const refused = turns.reduce((n, t) => n + t.rejections.length, 0);
  const acts = turns.reduce((n, t) => n + t.calls.filter((c) => c.tool !== "end_turn").length, 0);
  const founded = turns.filter((t) => t.calls.some((c) => c.tool === "found_org" && c.ok)).length;
  const jobsTaken = turns.filter((t) => t.calls.some((c) => c.tool === "accept_offer" && c.ok)).length;
  const hungry = turns.filter((t) => t.situation.food < 20).length;
  const confusions = turns.reduce((n, t) => n + t.did_not_understand.length, 0);
  const endings = new Map<string, number>();
  for (const t of turns) endings.set(t.ended_by, (endings.get(t.ended_by) ?? 0) + 1);
  const skipped = records.filter((r) => r.kind === "skip").length;
  const cost = turns.reduce((n, t) => n + t.usage.usd, 0) + records.filter((r) => r.kind === "reflection").reduce((n, r) => n + (r.kind === "reflection" ? r.usage.usd : 0), 0);
  return [
    `**${name}** (${first.persona}, ${first.brain}${first.model ? ` on ${first.model}` : ""}) took ${turns.length} turns` +
      (skipped ? ` and skipped ${skipped}` : "") +
      `, making ${acts} tool calls of which ${refused} were refused.`,
    `Started with ${cr(first.situation.balance)} credits, ${first.situation.jobs} job(s), ${first.situation.housed ? "housed" : "unhoused"}; ended with ${cr(last.situation.balance)} credits, ${last.situation.jobs} job(s), ${last.situation.housed ? "housed" : "unhoused"}, food ${last.situation.food.toFixed(0)}.`,
    (jobsTaken ? `Accepted ${jobsTaken} offer(s). ` : "") +
      (founded ? `Founded ${founded} org(s). ` : "") +
      (hungry ? `Was below 20 food in ${hungry} turn(s). ` : "") +
      (confusions ? `Reported ${confusions} thing(s) not understood. ` : "") +
      `Turns ended by: ${[...endings].map(([k, n]) => `${k} ${n}`).join(", ")}.` +
      (cost ? ` Cost $${cost.toFixed(2)}.` : ""),
  ].join(" ");
}

export function buildReport(f: Folded): string {
  const out: string[] = [];
  const s = f.summary;
  out.push(`# Playtest run ${f.run}`, "");
  out.push(`Synthetic cohort (S1.16, ADR-0009/0010). ${f.players.size} players, ${f.turns.length} turns` + (s ? `, stopped by ${s.stopped_by}, ${s.skipped} turns skipped over ${s.ticks_seen} hours and ${s.cycles_seen} day ends.` : "."), "");
  if (s) {
    out.push("## Run", "", "| player | persona | brain | model | skipped |", "|---|---|---|---|---|");
    for (const p of s.players) out.push(`| ${p.name} | ${p.persona} | ${p.brain} | ${p.model ?? "-"} | ${p.skipped} |`);
    const b = s.budget;
    out.push("", `Spend: $${b.total.usd.toFixed(2)} of $${b.max_usd} (${b.tokens.toLocaleString()} of ${b.max_tokens.toLocaleString()} tokens).`);
    const byModel = Object.entries(b.by_model);
    if (byModel.length) {
      out.push("", "| model | input | cache read | cache write | output | usd |", "|---|---|---|---|---|---|");
      for (const [m, u] of byModel) out.push(`| ${m} | ${u.input} | ${u.cache_read} | ${u.cache_write} | ${u.output} | ${u.usd.toFixed(2)} |`);
    }
    out.push("");
  }

  out.push("## The economy at the end", "");
  if (f.stats) {
    const live = (f.stats.live ?? {}) as Record<string, unknown>;
    const clock = (f.stats.clock ?? {}) as Record<string, unknown>;
    out.push(
      `Epoch ${String(clock.epoch)}, day ${String(clock.cycle)}, hour ${String(clock.tick)}. Population ${String(live.population)}, ${String(live.active_humans)} people, ${String(live.unemployed)} unemployed. Firms ${String(f.stats.firm_count)}; credit outstanding ${cr(Number(f.stats.credit_outstanding ?? 0))}; food last ${live.food_last_price != null ? cr(Number(live.food_last_price)) : "?"}; price index ${live.price_index != null ? Number(live.price_index).toFixed(2) : "?"}.`,
    );
    const last = f.stats.last_cycle as Record<string, unknown> | null | undefined;
    if (last) {
      out.push("", "Last day's aggregates:", "", "```json", JSON.stringify(last, null, 1).slice(0, 3000), "```");
    }
  } else out.push("(no /stats snapshot)");
  out.push("");
  if (f.rollover) {
    const r = f.rollover;
    const windowSecs = Math.round((new Date(String(r.closes_at)).getTime() - new Date(String(r.ended_at)).getTime()) / 1000);
    out.push(
      "## The epoch's end",
      "",
      `Epoch ${String(r.archived_epoch)} ended (${String(r.reason)}) after day ${String(r.final_cycle)} and was archived with ${String(r.statements)} closing statement${Number(r.statements) === 1 ? "" : "s"}${r.statement_by ? ` (one by ${String(r.statement_by)})` : ""}; the window was ${windowSecs} s. ${r.next_epoch_at ? `Epoch ${String(r.next_epoch)} started on its own at ${String(r.next_epoch_at)}.` : "The next epoch did not start within the wait."}`,
      "",
    );
  }

  // Rejections
  const groups = new Map<string, Map<string, Rejection>>();
  for (const t of f.turns) {
    for (const r of t.rejections) {
      const byText = groups.get(r.code) ?? new Map<string, Rejection>();
      groups.set(r.code, byText);
      const key = normalise(r.detail);
      const row = byText.get(key) ?? { code: r.code, text: key, count: 0, players: new Set(), tools: new Set(), example: r.detail };
      row.count += 1;
      row.players.add(t.player);
      row.tools.add(r.tool);
      byText.set(key, row);
    }
  }
  out.push("## Rejections", "");
  if (groups.size === 0) out.push("None.");
  for (const [code, byText] of [...groups].sort((a, b) => sum(b[1]) - sum(a[1]))) {
    out.push(`### ${code} (${sum(byText)})`, "", "| count | text | tools | players |", "|---|---|---|---|");
    for (const r of [...byText.values()].sort((a, b) => b.count - a.count)) {
      out.push(`| ${r.count} | ${md(r.example)} | ${[...r.tools].join(", ")} | ${[...r.players].join(", ")} |`);
    }
    out.push("");
  }

  // Defects from the fuzzer and confusions from the models
  const defects: { player: string; tick: string; text: string }[] = [];
  const copy = new Map<string, { player: string; tick: string; text: string }>();
  const confusions = new Map<string, { player: string; tick: string; text: string }[]>();
  for (const t of f.turns) {
    for (const d of t.did_not_understand) {
      const where = `day ${t.cycle} hour ${t.tick}`;
      if (d.startsWith("DEFECT:")) defects.push({ player: t.player, tick: where, text: d.slice(7).trim() });
      else if (d.startsWith("COPY:")) {
        const text = d.slice(5).trim();
        if (!copy.has(normalise(text))) copy.set(normalise(text), { player: t.player, tick: where, text });
      } else {
        const k = endpointOf(d);
        const list = confusions.get(k) ?? [];
        list.push({ player: t.player, tick: where, text: d });
        confusions.set(k, list);
      }
    }
  }
  out.push("## Fuzzer findings", "");
  if (defects.length === 0) out.push("None: every probe was refused, without a server error and without raw ids in the refusal.");
  else {
    out.push("| player | when | finding |", "|---|---|---|");
    for (const d of defects) out.push(`| ${d.player} | ${d.tick} | ${md(d.text)} |`);
  }
  if (copy.size > 0) {
    out.push("", `Refusals that name things by raw id (${copy.size} distinct; copy, not rules):`, "", "| first seen | text |", "|---|---|");
    for (const c of copy.values()) out.push(`| ${c.player}, ${c.tick} | ${md(c.text)} |`);
  }
  out.push("", "## What players did not understand", "");
  if (confusions.size === 0) out.push("Nothing reported.");
  for (const [k, list] of [...confusions].sort((a, b) => b[1].length - a[1].length)) {
    out.push(`### ${k} (${list.length})`, "");
    for (const c of list) out.push(`- ${c.player}, ${c.tick}: ${md(c.text)}`);
    out.push("");
  }

  out.push("## Governance", "", governanceParagraph(f.turns), "");
  out.push(...jevSection(f.turns));
  out.push("## Each player's arc", "");
  for (const [name, records] of f.players) out.push(arc(name, records), "");

  // Harness notes
  const odd = f.turns.filter((t) => t.ended_by === "text" || t.ended_by === "cap" || t.ended_by === "error");
  out.push("## Harness notes", "");
  if (odd.length === 0) out.push("Every turn ended with end_turn (or a script).");
  else {
    const by = new Map<string, number>();
    for (const t of odd) by.set(t.ended_by, (by.get(t.ended_by) ?? 0) + 1);
    out.push(`${odd.length} turn(s) ended without end_turn: ${[...by].map(([k, n]) => `${k} ${n}`).join(", ")}.`);
    for (const t of odd.filter((x) => x.error).slice(0, 10)) out.push(`- ${t.player}, day ${t.cycle} hour ${t.tick}: ${md(t.error ?? "")}`);
  }
  out.push("");
  return out.join("\n");
}

function sum(m: Map<string, Rejection>): number {
  let n = 0;
  for (const r of m.values()) n += r.count;
  return n;
}

function md(s: string): string {
  return s.replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
}
