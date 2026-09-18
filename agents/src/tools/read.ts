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
  "Per-hour volume-weighted prices per instrument over a window of hours (default one day).",
  z.object({ window: z.number().int().min(1).max(500).optional().describe("hours of history") }),
  async (i, ctx) => {
    const v = unwrap(await ctx.client.GET("/s/{id}/prices", { params: { path: { id: ctx.sid }, query: i.window === undefined ? {} : { window: i.window } } }));
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
];
