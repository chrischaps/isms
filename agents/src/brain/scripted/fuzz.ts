// The rule-prober: one probe an hour of something the rules say cannot be done,
// through the same tools. A probe expects a refusal. Two things are defects and
// go to the journal's `did_not_understand` with a `DEFECT:` prefix so the report
// can gather them and CI can fail on them: the server accepted it, or the server
// fell over (5xx). A refusal that names things by raw id (`c41`, `w19`) where a
// person would expect a name is a copy finding, prefixed `COPY:`, reported but
// not fatal. Each probe's expectation is stated, so a wrong expectation is a
// finding about the harness, not the game.

import * as act from "../../tools/act.ts";
import { invoke, type AnyTool, type ToolResult } from "../../tools/context.ts";
import { currentJobs, ensurePlan, jobOffers, takeAJob, workFullHours, type Script } from "./script.ts";

export type Probe = {
  name: string;
  /** Why this should be refused (the rule being pressed on). */
  rule: string;
  run: (s: Script) => Promise<{ tool: AnyTool; input: unknown } | null>;
};

const RAW_ID = /\b[cwod]\d+\b|\borg #\d+\b/;

export const PROBES: Probe[] = [
  {
    name: "hours past the budget",
    rule: "SetLabor must keep the sum of hours within the day's budget",
    run: async (s) => {
      const j = currentJobs(s.home)[0];
      return j ? { tool: act.setLabor, input: { allocations: [{ workplace: j.workplace, hours: 24, effort: "high" }] } } : null;
    },
  },
  {
    name: "work where I am not employed",
    rule: "SetLabor names only workplaces the citizen has a contract at",
    run: async () => ({ tool: act.setLabor, input: { allocations: [{ workplace: 999_999, hours: 4, effort: "normal" }] } }),
  },
  {
    name: "a good that does not exist",
    rule: "PlaceOrder refuses an unknown instrument",
    run: async () => ({ tool: act.placeOrder, input: { instrument: "gold", side: "bid", qty: 1, limit_price: 100 } }),
  },
  {
    name: "a bid I cannot afford",
    rule: "PlaceOrder escrows money; a bid beyond the balance is refused",
    run: async (s) => ({ tool: act.placeOrder, input: { instrument: "food", side: "bid", qty: 1_000_000, limit_price: Math.max(1, s.balance) } }),
  },
  {
    name: "an ask of goods I do not hold",
    rule: "PlaceOrder escrows goods; an ask beyond the pantry is refused",
    run: async () => ({ tool: act.placeOrder, input: { instrument: "machines", side: "ask", qty: 50, limit_price: 1 } }),
  },
  {
    name: "accept an offer that does not exist",
    rule: "AcceptOffer of an unknown id is refused",
    run: async () => ({ tool: act.acceptOffer, input: { offer: 999_999, on_behalf_of: null } }),
  },
  {
    name: "accept my own job offer twice",
    rule: "A citizen holds at most one contract per offer",
    run: async (s) => {
      const j = currentJobs(s.home)[0];
      const same = jobOffers(await s.board()).find((o) => j && o.org === j.org && o.workplace === j.workplace);
      return same ? { tool: act.acceptOffer, input: { offer: same.id, on_behalf_of: null } } : null;
    },
  },
  {
    name: "found a firm with an empty pantry",
    rule: "FoundOrg needs the fee and the Materials",
    run: async (s) => ((s.pantry.materials ?? 0) < 20 ? { tool: act.foundOrg, input: { kind: "firm", name: "Nothing & Co", first_workplace: { kind: "mine", slot: null } } } : null),
  },
  {
    name: "transfer to myself",
    rule: "Transfer to oneself is meaningless and should be refused",
    run: async (s) => ({ tool: act.transfer, input: { to: { citizen: s.home.citizen.id }, asset: { money: 1 }, memo: "to me", on_behalf_of: null } }),
  },
  {
    name: "transfer more than I have",
    rule: "Transfer cannot overdraw",
    run: async (s) => ({ tool: act.transfer, input: { to: { citizen: 0 }, asset: { money: s.balance + 1_000_000 }, memo: "too much", on_behalf_of: null } }),
  },
  {
    name: "move into a house that is not mine",
    rule: "MoveIn needs ownership or a lease",
    run: async () => ({ tool: act.moveIn, input: { dwelling: 999_999 } }),
  },
  {
    name: "move out when not housed",
    rule: "MoveOut of a dwelling one does not occupy is refused",
    run: async (s) => (s.home.household.dwelling ? null : { tool: act.moveOut, input: { dwelling: 1 } }),
  },
  {
    name: "cancel an order that is not mine",
    rule: "CancelOrder is the owner's alone",
    run: async () => ({ tool: act.cancelOrder, input: { order: 1 } }),
  },
  {
    name: "terminate a contract I am not party to",
    rule: "Terminate is for parties only",
    run: async () => ({ tool: act.terminateContract, input: { contract: 1, on_behalf_of: null } }),
  },
  {
    name: "declare a dividend on a firm I do not own",
    rule: "DeclareDividend is the controlling owner's alone",
    run: async (s) => {
      const orgs = await s.orgs();
      const other = orgs?.orgs.find((o) => !o.i_manage && o.kind === "firm");
      return other ? { tool: act.dividend, input: { org: other.id, per_share: 1 } } : null;
    },
  },
  {
    name: "post a job at a firm I do not manage",
    rule: "OfferEmployment is manager-only",
    run: async (s) => {
      const orgs = await s.orgs();
      const other = orgs?.orgs.find((o) => !o.i_manage && o.workplaces.length > 0);
      const wp = other?.workplaces[0];
      return other && wp
        ? { tool: act.offerEmployment, input: { org: other.id, workplace: wp.id, pay: { hourly: 1 }, max_hours: 8, places: 1, notice_cycles: 1, term_cycles: null } }
        : null;
    },
  },
  {
    name: "issue shares in a firm I do not own",
    rule: "IssueShares is the controlling owner's alone",
    run: async (s) => {
      const orgs = await s.orgs();
      const other = orgs?.orgs.find((o) => !o.i_manage && o.kind === "firm");
      return other ? { tool: act.issueShares, input: { org: other.id, qty: 1 } } : null;
    },
  },
  {
    name: "speak in an org channel I do not belong to",
    rule: "Org channels are for members",
    run: async (s) => {
      const orgs = await s.orgs();
      const other = orgs?.orgs.find((o) => !o.i_manage && o.my_shares === 0);
      return other ? { tool: act.postMessage, input: { channel: `org:${other.id}`, body: "hello?" } } : null;
    },
  },
  {
    name: "a standing order for shares in a firm that does not exist",
    rule: "SetPlan validates every standing order's instrument",
    run: async () => ({
      tool: act.setPlan,
      input: {
        labor: "explicit",
        keep_food_at_least: 24,
        max_food_price: null,
        buy_wares_when: null,
        keep_balance_at_least: 0,
        standing_orders: [{ instrument: { share: 999_999 }, side: "bid", qty: 1, limit_price: 100, refresh: "each_cycle" }],
      },
    }),
  },
  {
    name: "a lease on a dwelling I do not own",
    rule: "OfferLease needs ownership",
    run: async () => ({ tool: act.offerLease, input: { asset: { dwelling: 1 }, rent_per_cycle: 100, term_cycles: null, on_behalf_of: null } }),
  },
  {
    name: "credit I cannot fund",
    rule: "OfferCredit escrows the principal",
    run: async (s) => ({ tool: act.offerCredit, input: { principal: s.balance + 1_000_000, rate_per_cycle_bp: 100, term_cycles: 3, collateral: null, to: null, on_behalf_of: null } }),
  },
];

export function judge(probe: Probe, r: ToolResult): string | null {
  if (r.ok) return `DEFECT: accepted "${probe.name}" (${probe.rule})`;
  if (r.status >= 500) return `DEFECT: server error ${r.status} on "${probe.name}": ${r.detail}`;
  if (RAW_ID.test(r.detail)) return `COPY: raw ids in a refusal for "${probe.name}": "${r.detail}"`;
  return null;
}

/** Work enough to eat, then one probe per hour, round robin. */
export async function ruleProber(s: Script): Promise<string> {
  await ensurePlan(s);
  await takeAJob(s);
  await workFullHours(s);
  const i = ((s.memory.probe as number | undefined) ?? 0) % PROBES.length;
  s.memory.probe = i + 1;
  const probe = PROBES[i]!;
  const call = await probe.run(s);
  if (!call) return `probe "${probe.name}" not applicable this hour`;
  if (s.actionsLeft <= 0) return `no action left for probe "${probe.name}"`;
  const r = await invoke(call.tool, call.input, s.ctx);
  const verdict = judge(probe, r);
  if (verdict) s.note(verdict);
  const said = r.ok ? "ACCEPTED" : `refused ${r.code}: "${r.detail}"`;
  return `probed "${probe.name}" (${probe.rule}): ${said}`;
}
