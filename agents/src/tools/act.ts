// Acting in the game: each tool is one command on the public API, exactly as
// the web client sends it. Money is in cents. A refusal comes back as the
// server's problem document; `invoke` counts every act, accepted or refused.

import { z } from "zod";
import { unwrap } from "../api/client.ts";
import type { AnyTool, ToolContext, ToolDef, ToolResult } from "./context.ts";

function act<I>(
  name: string,
  description: string,
  schema: z.ZodType<I>,
  run: (input: I, ctx: ToolContext) => Promise<unknown>,
): ToolDef<I> {
  return { name, description, kind: "act", schema, run: async (i, c) => ({ ok: true, data: await run(i, c) }) as ToolResult };
}

const cents = z.number().int().min(0).describe("in cents (100 = 1 credit)");
const id = z.number().int().min(0);
const effort = z.enum(["low", "normal", "high"]);
const good = z.enum(["grain", "ore", "materials", "food", "wares", "machines"]);
const workplaceKind = z.enum(["farm", "mine", "foundry", "mill", "workshop", "machine_shop", "builder"]);

/** What a command did, kept small: the clock and the event kinds it produced. */
function committed(v: { clock: unknown; events: { kind: string; seq: number }[]; first_seq?: number | null }) {
  return { clock: v.clock, first_seq: v.first_seq ?? null, events: v.events.map((e) => `${e.kind}#${e.seq}`) };
}

export const setLabor = act(
  "set_labor",
  "Set how you work this day: up to two workplaces you are employed at, hours each (the sum within your budget), and effort. Replaces the whole allocation.",
  z.object({
    allocations: z.array(z.object({ workplace: id, hours: z.number().int().min(0).max(24), effort })).max(2),
  }),
  async (i, ctx) => committed(unwrap(await ctx.client.PUT("/s/{id}/labor", { params: { path: { id: ctx.sid } }, body: { allocations: i.allocations } }))),
);

export const setPlan = act(
  "set_plan",
  "Replace your standing plan: what the engine buys for you every hour while you are away. keep_food_at_least is units of Food; prices in cents; standing orders refresh each_tick or each_cycle.",
  z.object({
    labor: z.enum(["explicit", "accept_assignment", "follow_norm"]).default("explicit"),
    keep_food_at_least: z.number().int().min(0),
    max_food_price: cents.nullable().default(null).describe("null = last price x 1.25"),
    buy_wares_when: z.object({ comfort_below: z.number().int().min(0).max(100), balance_above: cents, max_price: cents.nullable() }).nullable().default(null),
    keep_balance_at_least: cents,
    standing_orders: z
      .array(
        z.object({
          instrument: z.union([z.object({ good }), z.object({ share: id })]),
          side: z.enum(["bid", "ask"]),
          qty: z.number().int().min(1),
          limit_price: cents,
          refresh: z.enum(["each_tick", "each_cycle"]),
        }),
      )
      .max(8)
      .default([]),
    vote_default: z
      .union([z.enum(["abstain", "none"]), z.object({ follow: id })])
      .default("abstain")
      .describe("how the assembly counts you at a close you missed: abstain, none (no ballot), or follow a citizen's ballot"),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.PUT("/s/{id}/plan", {
          params: { path: { id: ctx.sid } },
          body: { plan: { ...i } as never },
        }),
      ),
    ),
);

export const placeOrder = act(
  "place_order",
  "Place a limit order on a book. A bid escrows money, an ask escrows the goods or shares. Expires at the end of the next day unless given.",
  z.object({
    instrument: z.string().describe("food, wares, grain, ore, materials, machines, or share:<org id>"),
    side: z.enum(["bid", "ask"]),
    qty: z.number().int().min(1),
    limit_price: cents,
    on_behalf_of: id.optional().describe("an org you manage"),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/orders", {
          params: { path: { id: ctx.sid } },
          body: { instrument: i.instrument, side: i.side, qty: i.qty, limit_price: i.limit_price, on_behalf_of: i.on_behalf_of ?? null },
        }),
      ),
    ),
);

export const cancelOrder = act(
  "cancel_order",
  "Cancel one of your open orders and release its escrow.",
  z.object({ order: id, on_behalf_of: id.optional() }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.DELETE("/s/{id}/orders/{oid}", {
          params: { path: { id: ctx.sid, oid: i.order }, query: i.on_behalf_of === undefined ? {} : { on_behalf_of: i.on_behalf_of } },
        }),
      ),
    ),
);

export const foundOrg = act(
  "found_org",
  "Found an organisation (a firm in Freeport). Costs the founding fee and Materials from your pantry; see orgs.founding. You become sole owner and manager.",
  z.object({
    kind: z.enum(["firm", "association"]).default("firm"),
    name: z.string().min(1).max(40),
    first_workplace: z.object({ kind: workplaceKind, slot: id.nullable().default(null) }).nullable().default(null),
  }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.POST("/s/{id}/orgs", { params: { path: { id: ctx.sid } }, body: { kind: i.kind, name: i.name, first_workplace: i.first_workplace } }))),
);

export const offerEmployment = act(
  "post_employment_offer",
  "As a manager, post a job offer on the notice board for one of your workplaces: hourly pay or piece rate in cents, max hours a day, places, notice in days, optional term.",
  z.object({
    org: id,
    workplace: id,
    pay: z.union([z.object({ hourly: cents }), z.object({ piece_rate: cents })]),
    max_hours: z.number().int().min(1).max(24),
    places: z.number().int().min(1),
    notice_cycles: z.number().int().min(0).default(1),
    term_cycles: z.number().int().min(1).nullable().default(null),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/orgs/{oid}/offers", {
          params: { path: { id: ctx.sid, oid: i.org } },
          body: { workplace: i.workplace, pay: i.pay as never, max_hours: i.max_hours, places: i.places, notice_cycles: i.notice_cycles, term_cycles: i.term_cycles },
        }),
      ),
    ),
);

export const addWorkplace = act(
  "add_workplace",
  "As a manager, add a workplace to your org (costs Materials from the org's inventory; slot-limited kinds need a free slot).",
  z.object({ org: id, kind: workplaceKind, slot: id.nullable().default(null) }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.POST("/s/{id}/orgs/{oid}/workplaces", { params: { path: { id: ctx.sid, oid: i.org } }, body: { kind: i.kind, slot: i.slot } }))),
);

export const machines = act(
  "machines",
  "As a manager, install or uninstall machines (from the org's inventory) at a workplace.",
  z.object({ org: id, workplace: id, action: z.enum(["install", "uninstall"]), qty: z.number().int().min(1) }),
  async (i, ctx) =>
    committed(
      unwrap(await ctx.client.POST("/s/{id}/orgs/{oid}/machines", { params: { path: { id: ctx.sid, oid: i.org } }, body: { workplace: i.workplace, action: i.action, qty: i.qty } })),
    ),
);

export const dividend = act(
  "declare_dividend",
  "As the controlling owner, declare a dividend per share in cents, paid at the end of the day.",
  z.object({ org: id, per_share: cents }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/orgs/{oid}/dividend", { params: { path: { id: ctx.sid, oid: i.org } }, body: { per_share: i.per_share } }))),
);

export const issueShares = act(
  "issue_shares",
  "As the controlling owner, issue new shares to yourself (dilutes others).",
  z.object({ org: id, qty: z.number().int().min(1) }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/orgs/{oid}/shares", { params: { path: { id: ctx.sid, oid: i.org } }, body: { qty: i.qty } }))),
);

export const appointManager = act(
  "appoint_manager",
  "As the controlling owner, appoint a citizen as manager (null = yourself).",
  z.object({ org: id, citizen: id.nullable().default(null) }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/orgs/{oid}/manager", { params: { path: { id: ctx.sid, oid: i.org } }, body: { citizen: i.citizen } }))),
);

export const offerSale = act(
  "post_sale_offer",
  "Offer goods, shares, or a dwelling for sale on the notice board at a price in cents (or for goods), optionally to one citizen only.",
  z.object({
    asset: z.union([z.object({ good: z.tuple([good, z.number().int().min(1)]) }), z.object({ shares: z.tuple([id, z.number().int().min(1)]) }), z.object({ dwelling: id })]),
    price: z.union([z.object({ money: cents }), z.object({ good: z.tuple([good, z.number().int().min(1)]) })]),
    to: z.object({ citizen: id }).nullable().default(null),
    on_behalf_of: id.nullable().default(null),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/offers/sale", {
          params: { path: { id: ctx.sid } },
          body: { asset: i.asset as never, price: i.price as never, to: i.to as never, on_behalf_of: i.on_behalf_of },
        }),
      ),
    ),
);

export const postWanted = act(
  "post_wanted",
  "Post a wanted ad: a good, quantity, and the most you would pay per unit in cents.",
  z.object({ good, qty: z.number().int().min(1), max_price: cents }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/offers/wanted", { params: { path: { id: ctx.sid } }, body: { good: i.good, qty: i.qty, max_price: i.max_price } }))),
);

export const offerCredit = act(
  "post_credit_offer",
  "Offer a loan: principal in cents, interest per day in basis points (100 = 1%), term in days, optional collateral (a dwelling or shares) and optional borrower. Repaid in equal daily installments; a missed one seizes collateral, then flags default.",
  z.object({
    principal: cents,
    rate_per_cycle_bp: z.number().int().min(0),
    term_cycles: z.number().int().min(1),
    collateral: z.union([z.object({ dwelling: id }), z.object({ shares: z.tuple([id, z.number().int().min(1)]) })]).nullable().default(null),
    to: z.union([z.object({ citizen: id }), z.object({ org: id })]).nullable().default(null),
    on_behalf_of: id.nullable().default(null),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/offers/credit", {
          params: { path: { id: ctx.sid } },
          body: {
            principal: i.principal,
            rate_per_cycle_bp: i.rate_per_cycle_bp,
            term_cycles: i.term_cycles,
            collateral: i.collateral as never,
            to: i.to as never,
            on_behalf_of: i.on_behalf_of,
          },
        }),
      ),
    ),
);

export const offerLease = act(
  "post_lease_offer",
  "As an owner, offer a dwelling (or workplace) for lease at a rent per day in cents; the tenant moves in on accepting.",
  z.object({
    asset: z.union([z.object({ dwelling: id }), z.object({ workplace: id })]),
    rent_per_cycle: cents,
    term_cycles: z.number().int().min(1).nullable().default(null),
    on_behalf_of: id.nullable().default(null),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/offers/lease", {
          params: { path: { id: ctx.sid } },
          body: { asset: i.asset as never, rent_per_cycle: i.rent_per_cycle, term_cycles: i.term_cycles, on_behalf_of: i.on_behalf_of },
        }),
      ),
    ),
);

export const acceptOffer = act(
  "accept_offer",
  "Accept an offer from the notice board by id: take a job, buy what is for sale, take a loan, rent a dwelling.",
  z.object({ offer: id, on_behalf_of: id.nullable().default(null).describe("an org you manage, when the org is the party") }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.POST("/s/{id}/offers/{oid}/accept", { params: { path: { id: ctx.sid, oid: i.offer } }, body: { on_behalf_of: i.on_behalf_of } }))),
);

export const withdrawOffer = act(
  "withdraw_offer",
  "Withdraw an open offer you posted: a job, a loan, a lease, a sale or a wanted ad. Accepted contracts are untouched.",
  z.object({ offer: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.DELETE("/s/{id}/offers/{oid}", { params: { path: { id: ctx.sid, oid: i.offer } } }))),
);

export const terminateContract = act(
  "terminate_contract",
  "End a contract you are party to (a job with notice, a lease). Credit cannot be terminated; it is repaid.",
  z.object({ contract: id, on_behalf_of: id.nullable().default(null) }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/contracts/{cid}/terminate", { params: { path: { id: ctx.sid, cid: i.contract } }, body: { on_behalf_of: i.on_behalf_of } }),
      ),
    ),
);

export const transfer = act(
  "transfer",
  "Give money or goods to a citizen or an org, with a memo.",
  z.object({
    to: z.union([z.object({ citizen: id }), z.object({ org: id })]),
    asset: z.union([z.object({ money: cents }), z.object({ good: z.tuple([good, z.number().int().min(1)]) })]),
    memo: z.string().max(140).default(""),
    on_behalf_of: id.nullable().default(null),
  }),
  async (i, ctx) =>
    committed(
      unwrap(
        await ctx.client.POST("/s/{id}/transfers", {
          params: { path: { id: ctx.sid } },
          body: { to: i.to as never, asset: i.asset as never, memo: i.memo, on_behalf_of: i.on_behalf_of },
        }),
      ),
    ),
);

export const moveIn = act(
  "move_in",
  "Move into a dwelling you own or lease.",
  z.object({ dwelling: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/dwellings/{did}/move-in", { params: { path: { id: ctx.sid, did: i.dwelling } } }))),
);

export const moveOut = act(
  "move_out",
  "Move out of your dwelling.",
  z.object({ dwelling: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/dwellings/{did}/move-out", { params: { path: { id: ctx.sid, did: i.dwelling } } }))),
);

export const postMessage = act(
  "post_message",
  "Say something on the Square or in an org channel (at most one message every two seconds).",
  z.object({ channel: z.string().describe("square or org:<org id>"), body: z.string().min(1).max(500) }),
  async (i, ctx) => unwrap(await ctx.client.POST("/s/{id}/channels/{channel}/messages", { params: { path: { id: ctx.sid, channel: i.channel } }, body: { body: i.body } })),
);

// -- the assembly, offices, the coordinator's powers, positions (S2.10) --------

const office = z.string().describe("coordinator, planning_committee, legislator, union_steward or bank_board");
const proposalKind = z.union([
  z.literal("resolution").describe("a resolution: its text is the motion"),
  z.object({
    policy_change: z.object({
      patch: z
        .object({
          work_norm_hours: z.number().int().min(0).max(24).optional(),
          rationing: z.enum(["need_first", "equal_shortfall", "lottery"]).optional().describe("a coordinator's to propose"),
          monitoring: z.enum(["high", "medium", "low"]).optional(),
          materials_split: z.object({ wares: z.number(), machines: z.number(), dwellings: z.number() }).optional(),
        })
        .passthrough()
        .describe("only the fields you move; the rest stay as they are"),
    }),
  }),
  z.object({ honor: z.object({ citizen: id }) }),
  z.object({ recall: z.object({ office, citizen: id }) }),
  z.object({ election: z.object({ office }) }).describe("never accepted: elections open by the calendar; stand instead"),
]);

export const propose = act(
  "propose",
  "Open a proposal before the assembly; it closes at the end of this day. Kinds: resolution (the text is the motion), policy_change (a patch of the policy: work_norm_hours, rationing, monitoring, materials_split), honor (a citizen), recall (an office-holder). Elections are not proposed: stand for the office.",
  z.object({ title: z.string().min(1).max(80), text: z.string().max(2000).default(""), kind: proposalKind }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.POST("/s/{id}/proposals", { params: { path: { id: ctx.sid } }, body: { title: i.title, text: i.text, kind: i.kind as never } }))),
);

export const vote = act(
  "vote",
  "Cast or replace your ballot on an open proposal: yes, no or abstain. The roll is public.",
  z.object({ proposal: id, ballot: z.enum(["yes", "no", "abstain"]) }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.PUT("/s/{id}/proposals/{pid}/ballot", { params: { path: { id: ctx.sid, pid: i.proposal } }, body: { ballot: i.ballot } }))),
);

export const stand = act(
  "stand",
  "Stand in the open election for an office. Seats are filled at the day's end by approvals.",
  z.object({ office }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/offices/{kind}/candidacy", { params: { path: { id: ctx.sid, kind: i.office } } }))),
);

export const withdrawCandidacy = act(
  "withdraw_candidacy",
  "Withdraw your candidacy for an office.",
  z.object({ office }),
  async (i, ctx) => committed(unwrap(await ctx.client.DELETE("/s/{id}/offices/{kind}/candidacy", { params: { path: { id: ctx.sid, kind: i.office } } }))),
);

export const approve = act(
  "approve",
  "Cast or replace your approval ballot in an office's election: any subset of the candidates' citizen ids (an empty list approves nobody).",
  z.object({ office, candidates: z.array(id).max(20) }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.PUT("/s/{id}/offices/{kind}/ballot", { params: { path: { id: ctx.sid, kind: i.office } }, body: { candidates: i.candidates } }))),
);

export const publishPlan = act(
  "publish_plan",
  "As a coordinator, publish the Plan: a target in units a day per workplace id (see published_plan). Advisory under a direct assembly.",
  z.object({ targets: z.record(z.string(), z.number().min(0)) }),
  async (i, ctx) =>
    committed(unwrap(await ctx.client.PUT("/s/{id}/offices/coordinator/plan", { params: { path: { id: ctx.sid } }, body: { targets: i.targets } }))),
);

export const openWorkplace = act(
  "open_workplace",
  "As a coordinator, open a workplace of the collective on a free land slot of its kind (any, when slot is null); the founding Materials come out of the Common Store.",
  z.object({ kind: workplaceKind, slot: id.nullable().default(null) }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/workplaces", { params: { path: { id: ctx.sid } }, body: { kind: i.kind, slot: i.slot } }))),
);

export const closeWorkplace = act(
  "close_workplace",
  "As a coordinator, close a workplace of the collective: its workers lose the hour and its slot is freed.",
  z.object({ workplace: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.DELETE("/s/{id}/workplaces/{wid}", { params: { path: { id: ctx.sid, wid: i.workplace } } }))),
);

export const postFloor = act(
  "post_floor",
  "Speak on a proposal's floor (open while it is, and for one day after it closes).",
  z.object({ proposal: id, body: z.string().min(1).max(500) }),
  async (i, ctx) =>
    unwrap(await ctx.client.POST("/s/{id}/channels/{channel}/messages", { params: { path: { id: ctx.sid, channel: `assembly:${i.proposal}` } }, body: { body: i.body } })),
);

export const takePosition = act(
  "take_position",
  "Under a work norm: take a position at a workplace (no contract; see ledger for the least-staffed one and the caps). Then set_labor names it.",
  z.object({ workplace: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.POST("/s/{id}/workplaces/{wid}/position", { params: { path: { id: ctx.sid, wid: i.workplace } } }))),
);

export const leavePosition = act(
  "leave_position",
  "Give up a norm position at a workplace.",
  z.object({ workplace: id }),
  async (i, ctx) => committed(unwrap(await ctx.client.DELETE("/s/{id}/workplaces/{wid}/position", { params: { path: { id: ctx.sid, wid: i.workplace } } }))),
);

export const ACT_TOOLS: AnyTool[] = [
  setLabor,
  setPlan,
  placeOrder,
  cancelOrder,
  foundOrg,
  offerEmployment,
  addWorkplace,
  machines,
  dividend,
  issueShares,
  appointManager,
  offerSale,
  postWanted,
  offerCredit,
  offerLease,
  acceptOffer,
  withdrawOffer,
  terminateContract,
  transfer,
  moveIn,
  moveOut,
  postMessage,
  propose,
  vote,
  stand,
  withdrawCandidacy,
  approve,
  publishPlan,
  openWorkplace,
  closeWorkplace,
  postFloor,
  takePosition,
  leavePosition,
];
