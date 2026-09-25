// Reading the game: each tool is one GET on the public API, returned as the
// server sent it (long lists cut to the newest rows, and the cut named).
// Reads never count against the action cap.

import { z } from "zod";
import { unwrap } from "../api/client.ts";
import { capRows, type AnyTool, type ToolContext, type ToolDef, type ToolResult } from "./context.ts";

const none = z.object({});
type None = z.infer<typeof none>;

function read<I>(
  name: string,
  description: string,
  schema: z.ZodType<I>,
  run: (input: I, ctx: ToolContext) => Promise<unknown>,
): ToolDef<I> {
  return { name, description, kind: "read", schema, run: async (i, c) => ({ ok: true, data: await run(i, c) }) as ToolResult };
}

const path = (ctx: ToolContext) => ({ path: { id: ctx.sid } });

export const home = read<None>(
  "home",
  "Your situation: needs (food, shelter, comfort 0-100), pantry, dwelling, balance in cents, your jobs and labor allocations, your standing plan, the latest headlines, and what touched you since you last looked.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/home", { params: path(ctx) })),
);

export const plan = read<None>(
  "plan",
  "Your standing plan as the engine holds it (what it buys for you every hour while you are away).",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/plan", { params: path(ctx) })),
);

export const payslips = read<None>(
  "payslips",
  "Your payslips (Paid events), newest last, each with an explain of hours, rate and rule.",
  none,
  async (_, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/payslips", { params: path(ctx) }));
    return { clock: v.clock, ...capRows(v.payslips) };
  },
);

export const books = read<None>(
  "books",
  "Every order book: instrument (food, wares, grain, ore, materials, machines, or share:<org id>), best bid and ask in cents, depth, last price, and the price index.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/books", { params: path(ctx) })),
);

export const book = read(
  "book",
  "One order book in depth: bids, asks, your open orders, and the recent trade tape.",
  z.object({ instrument: z.string().describe("food, wares, grain, ore, materials, machines, or share:<org id>") }),
  async (i, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/books/{instrument}", { params: { path: { id: ctx.sid, instrument: i.instrument } } }));
    return { ...v, tape: capRows(v.tape).rows };
  },
);

export const prices = read(
  "prices",
  "Per-hour volume-weighted prices per instrument over a window of hours (default one day); the window runs back across an epoch's end unless an epoch is named.",
  z.object({
    window: z.number().int().min(1).max(500).optional().describe("hours of history"),
    epoch: z.number().int().min(1).optional().describe("read inside this epoch only (1 is the first)"),
  }),
  async (i, ctx) => {
    const query: { window?: number; epoch?: number } = {};
    if (i.window !== undefined) query.window = i.window;
    if (i.epoch !== undefined) query.epoch = i.epoch;
    const v = unwrap(await ctx.client.GET("/s/{id}/prices", { params: { path: { id: ctx.sid }, query } }));
    return { clock: v.clock, window: v.window, ...capRows(v.points, 120) };
  },
);

export const orgs = read<None>(
  "orgs",
  "Every organisation: kind, name, manager, treasury, inventory, workplaces with workers and machines, book value, your shares; plus the recipes (what each workplace kind makes and consumes), what founding costs, and land slot scarcity.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/orgs", { params: path(ctx) })),
);

export const org = read(
  "org",
  "One organisation in detail.",
  z.object({ org: z.number().int() }),
  async (i, ctx) => unwrap(await ctx.client.GET("/s/{id}/orgs/{oid}", { params: { path: { id: ctx.sid, oid: i.org } } })),
);

export const orgLedger = read(
  "org_ledger",
  "What moved an organisation's treasury (managers and owners only).",
  z.object({ org: z.number().int() }),
  async (i, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/orgs/{oid}/ledger", { params: { path: { id: ctx.sid, oid: i.org } } }));
    return { clock: v.clock, ...capRows(v.entries) };
  },
);

export const noticeBoard = read<None>(
  "notice_board",
  "Open offers anyone may accept: employment (job offers with pay, hours, places), sale, wanted, credit, lease. Each has an id to accept.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/notice-board", { params: path(ctx) })),
);

export const contracts = read<None>(
  "contracts",
  "Your contracts: employment, credit, leases; status and terms; each has an id to terminate.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/contracts", { params: path(ctx) })),
);

export const chronicle = read(
  "chronicle",
  "The Chronicle: one day's headlines (the current day by default).",
  z.object({ cycle: z.number().int().min(1).optional().describe("1-based day") }),
  async (i, ctx) =>
    unwrap(await ctx.client.GET("/s/{id}/chronicle", { params: { path: { id: ctx.sid }, query: i.cycle === undefined ? {} : { cycle: i.cycle } } })),
);

export const stats = read<None>(
  "stats",
  "The society's numbers: population, unemployment, price index, food price, firm count, credit outstanding, last day's aggregates.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/stats", { params: path(ctx) })),
);

export const citizens = read<None>(
  "citizens",
  "Every citizen: id, handle, kind (human or householder), flags such as hardship or default.",
  none,
  async (_, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/citizens", { params: path(ctx) }));
    return { clock: v.clock, ...capRows(v.citizens, 80) };
  },
);

export const scoreboard = read<None>(
  "scoreboard",
  "Net worth and self-made (net worth minus endowment) per citizen, with firm valuations.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/scoreboard", { params: path(ctx) })),
);

export const explain = read(
  "explain",
  "Why a number is what it is: the event at a log position with every Explain inside it.",
  z.object({ seq: z.number().int().min(0) }),
  async (i, ctx) => unwrap(await ctx.client.GET("/s/{id}/explain/{seq}", { params: { path: { id: ctx.sid, seq: i.seq } } })),
);

export const messages = read(
  "messages",
  "Read the Square or an org channel.",
  z.object({ channel: z.string().describe("square or org:<org id>") }),
  async (i, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/channels/{channel}/messages", { params: { path: { id: ctx.sid, channel: i.channel } } }));
    return { ...v, messages: capRows(v.messages).rows };
  },
);

// -- the assembly, the Common Store, the Ledger, the Plan (S2.10) --------------

export const proposals = read<None>(
  "proposals",
  "The assembly: open proposals (id, title, what each would do, the tally so far, the roll, your ballot, whether the floor takes posts) and those closed this epoch with their outcomes; the electorate, the quorum fraction and how many you may keep open.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/proposals", { params: path(ctx) })),
);

export const proposal = read(
  "proposal",
  "One proposal in full, open or closed this epoch.",
  z.object({ proposal: z.number().int().min(0) }),
  async (i, ctx) => unwrap(await ctx.client.GET("/s/{id}/proposals/{pid}", { params: { path: { id: ctx.sid, pid: i.proposal } } })),
);

export const offices = read<None>(
  "offices",
  "Every office the constitution seats: its rule (seats, term, recall), who sits and through which day, whether you hold it, and the election open for it (candidates with approvals, your approvals, whether you stand, and why you could not).",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/offices", { params: path(ctx) })),
);

export const store = read<None>(
  "store",
  "The Common Store: each shelf's stock, this hour's requests, what you may still draw (your entitlement) and your pending draw, the share each citizen would get if the day ended now; the rationing rule in force; today and yesterday as asked / served / short / shared; your draw record.",
  none,
  async (_, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/store", { params: path(ctx) }));
    return { ...v, my_draws: capRows(v.my_draws).rows };
  },
);

export const ledger = read<None>(
  "ledger",
  "The Ledger of Contribution: the work norm in hours, the monitoring level and its sigma, and every citizen's row (hours today and yesterday exact, output as attributed, the norm met or not, honors, workplaces; your row marked), plus the least-staffed workplace and the position caps.",
  none,
  async (_, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/ledger", { params: path(ctx) }));
    return { ...v, ...capRows(v.rows, 80) };
  },
);

export const publishedPlan = read<None>(
  "published_plan",
  "The Plan as published: every workplace's target beside yesterday's output and today's so far, workers and machines; the land slots and which are free; what a new workplace costs in Materials against the Store's; whether you coordinate.",
  none,
  async (_, ctx) => unwrap(await ctx.client.GET("/s/{id}/plan/published", { params: path(ctx) })),
);

export const READ_TOOLS: AnyTool[] = [
  home,
  plan,
  payslips,
  books,
  book,
  prices,
  orgs,
  org,
  orgLedger,
  noticeBoard,
  contracts,
  chronicle,
  stats,
  citizens,
  scoreboard,
  explain,
  messages,
  proposals,
  proposal,
  offices,
  store,
  ledger,
  publishedPlan,
];
