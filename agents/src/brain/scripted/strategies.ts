// The scripted strategies: what each persona does when no model is playing it.
// They use the same tools as the model and leave the same journal. Each returns
// its intent in one line. Anything the goals imply that householders never do
// (switching jobs, holding stock, founding) is here on purpose.

import * as act from "../../tools/act.ts";
import * as read from "../../tools/read.ts";
import {
  currentJobs,
  ensurePlan,
  isRefusal,
  jobOffers,
  lastPrice,
  median,
  takeAJob,
  workFullHours,
  type Script,
} from "./script.ts";

export type Strategy = (s: Script) => Promise<string>;

/** What a householder does: a job when unemployed, full hours, Food kept up. The floor under every persona. */
export const householderLike: Strategy = async (s) => {
  await ensurePlan(s);
  const job = await takeAJob(s);
  const worked = await workFullHours(s);
  if (job) return `took job ${job.id} at ${job.hourly} an hour and worked it`;
  return worked ? "worked my hours" : "no job open; waited";
};

/** Compare every offer each hour; switch when one pays clearly more. */
export const wageMaximiser: Strategy = async (s) => {
  await ensurePlan(s);
  const jobs = currentJobs(s.home);
  const offers = jobOffers(await s.board());
  const best = offers[0];
  if (jobs.length === 0) {
    if (best && (await s.do(act.acceptOffer, { offer: best.id }))) {
      s.memory.acceptedAt = s.clock.engine_tick;
      return `unemployed; took the best offer ${best.id} at ${best.hourly}/h`;
    }
    return "unemployed and no offer to take";
  }
  const mine = Math.max(...jobs.map((j) => j.hourly));
  if (best && best.hourly > mine * 1.1 && !jobs.some((j) => j.org === best.org && j.workplace === best.workplace)) {
    // Take the better one first; if a second job is allowed, give notice on the worse afterwards.
    if (await s.do(act.acceptOffer, { offer: best.id })) {
      const worst = jobs.reduce((a, b) => (a.hourly <= b.hourly ? a : b));
      await s.do(act.terminateContract, { contract: worst.contract });
      await workFullHours(s);
      return `switched: ${best.hourly}/h beats ${mine}/h; gave notice on contract ${worst.contract}`;
    }
    await workFullHours(s);
    return `wanted to switch to offer ${best.id} at ${best.hourly}/h but was refused; kept working`;
  }
  await workFullHours(s);
  return `no better offer than ${mine}/h; worked full hours`;
};

/** Buy cheap against a trailing mean, sell dear; never more than a third of cash in stock. */
export const speculator: Strategy = async (s) => {
  await ensurePlan(s, 24, 0);
  await takeAJob(s);
  await workFullHours(s);
  const books = await s.books();
  if (!books) return "could not read the books";
  const means = (s.memory.means ??= {}) as Record<string, number[]>;
  const goods = ["grain", "ore", "materials", "wares", "machines"];
  const held: Record<string, number> = {};
  for (const g of goods) held[g] = s.pantry[g] ?? 0;
  const stockValue = goods.reduce((sum, g) => sum + held[g]! * (lastPrice(books, g) ?? 0), 0);
  const cashLimit = Math.floor(s.balance / 3);
  const done: string[] = [];
  for (const g of goods) {
    const p = lastPrice(books, g);
    if (p === null) continue;
    const hist = (means[g] ??= []);
    hist.push(p);
    if (hist.length > 24) hist.shift();
    if (hist.length < 6) continue;
    const mean = hist.reduce((a, b) => a + b, 0) / hist.length;
    if (s.actionsLeft <= 0) break;
    if (p < mean * 0.9 && stockValue < cashLimit) {
      const qty = Math.max(1, Math.floor(Math.min(cashLimit - stockValue, s.balance / 4) / p));
      if (await s.do(act.placeOrder, { instrument: g, side: "bid", qty, limit_price: p })) done.push(`bid ${qty} ${g} @${p} (mean ${Math.round(mean)})`);
    } else if (p > mean * 1.1 && held[g]! > 0) {
      const qty = held[g]!;
      if (await s.do(act.placeOrder, { instrument: g, side: "ask", qty, limit_price: p })) done.push(`ask ${qty} ${g} @${p} (mean ${Math.round(mean)})`);
    }
  }
  return done.length ? done.join("; ") : "prices near their means; held";
};

/** Save toward a firm, buy the Materials, found it, hire, sell the output. A lighter founder than the model plays. */
export const founder: Strategy = async (s) => {
  await ensurePlan(s, 24, 0);
  await takeAJob(s);
  await workFullHours(s);
  const orgs = await s.orgs();
  if (!orgs) return "could not read the orgs";
  const mine = orgs.orgs.find((o) => o.i_manage);
  const books = await s.books();
  if (mine) {
    const done: string[] = [];
    // Hire when a workplace has room; sell what is in the inventory.
    const offers = jobOffers(await s.board());
    const wage = median(offers.map((o) => o.hourly)) || 800;
    const wp = mine.workplaces[0];
    if (wp && wp.workers.length < 2 && !s.memory.jobPosted) {
      if (await s.do(act.offerEmployment, { org: mine.id, workplace: wp.id, pay: { hourly: wage }, max_hours: 8, places: 2, notice_cycles: 1, term_cycles: null })) {
        s.memory.jobPosted = true;
        done.push(`posted a job at ${wage}/h`);
      }
    }
    const inv = mine.inventory as Record<string, number>;
    for (const [good, qty] of Object.entries(inv)) {
      if (good === "materials" || qty <= 0 || s.actionsLeft <= 0) continue;
      const p = lastPrice(books, good);
      if (p === null) continue;
      if (await s.do(act.placeOrder, { instrument: good, side: "ask", qty, limit_price: Math.round(p * 1.05), on_behalf_of: mine.id })) done.push(`asked ${qty} ${good} @${Math.round(p * 1.05)}`);
    }
    if (mine.treasury > 3 * wage * 8 * 2 && s.clock.tick === 1 && s.actionsLeft > 0) {
      if (await s.do(act.dividend, { org: mine.id, per_share: Math.floor(mine.treasury / 4 / Math.max(1, mine.my_shares)) })) done.push("declared a dividend");
    }
    return done.length ? done.join("; ") : `ran ${mine.name}`;
  }
  const need = orgs.founding.materials;
  const fee = orgs.founding.money;
  const have = s.pantry.materials ?? 0;
  const pm = lastPrice(books, "materials");
  if (have >= need) {
    if (s.balance < fee) return `have the Materials; saving the fee (${s.balance}/${fee})`;
    const name = `${s.handle}'s Works`;
    if (await s.do(act.foundOrg, { kind: "firm", name, first_workplace: { kind: "mine", slot: null } })) return `founded ${name} with a mine`;
    return "founding was refused";
  }
  if (pm !== null && s.balance > fee + pm * (need - have)) {
    const qty = need - have;
    if (await s.do(act.placeOrder, { instrument: "materials", side: "bid", qty, limit_price: Math.round(pm * 1.1) })) return `bid ${qty} materials @${Math.round(pm * 1.1)} toward founding`;
  }
  return `saving toward founding: ${s.balance} cash, ${have}/${need} materials`;
};

/** The saver, scripted: a job, a floor on the balance, nothing bought but Food. */
export const saver: Strategy = async (s) => {
  await ensurePlan(s, 24, Math.floor(s.balance * 0.8));
  await takeAJob(s);
  await workFullHours(s);
  return `worked; keeping ${Math.floor(s.balance * 0.8)} as a floor`;
};

/** The slacker, scripted: few hours at low effort unless Food is falling. */
export const slacker: Strategy = async (s) => {
  await ensurePlan(s, 20, 0);
  await takeAJob(s);
  const hours = s.food < 30 ? 8 : 4;
  await workFullHours(s, "low", hours);
  return `worked ${hours} hours at low effort (food ${s.food})`;
};

/** The borrower, scripted: take any credit on the board, then act the founder. */
export const borrower: Strategy = async (s) => {
  const board = await s.board();
  const credit = board.filter((o) => o.kind === "credit");
  if (credit.length > 0 && !s.memory.borrowed && s.actionsLeft > 0) {
    if (await s.do(act.acceptOffer, { offer: credit[0]!.id })) {
      s.memory.borrowed = true;
      return `took credit offer ${credit[0]!.id}`;
    }
  }
  return founder(s);
};

/** The landlord, scripted: buy a dwelling on sale when affordable, lease it out. */
export const landlord: Strategy = async (s) => {
  await ensurePlan(s, 24, 0);
  await takeAJob(s);
  await workFullHours(s);
  const board = await s.board();
  const owned = (s.memory.dwellings ??= []) as number[];
  for (const o of board) {
    if (o.kind !== "sale") continue;
    const sale = (o.body as { sale?: { asset?: { dwelling?: number }; price?: { money?: number } } }).sale;
    const did = sale?.asset?.dwelling;
    const price = sale?.price?.money;
    if (did === undefined || price === undefined || price > s.balance * 0.8 || s.actionsLeft <= 0) continue;
    if (await s.do(act.acceptOffer, { offer: o.id })) {
      owned.push(did);
      return `bought dwelling ${did} for ${price}`;
    }
  }
  const wages = jobOffers(board).map((j) => j.hourly);
  const rent = Math.max(100, Math.round((median(wages) || 800) * 8 * 0.2));
  for (const did of owned) {
    if (s.memory[`leased:${did}`] || s.actionsLeft <= 0) continue;
    if (await s.do(act.offerLease, { asset: { dwelling: did }, rent_per_cycle: rent, term_cycles: null, on_behalf_of: null })) {
      s.memory[`leased:${did}`] = true;
      return `offered dwelling ${did} to let at ${rent} a day`;
    }
  }
  const v = await s.get<unknown>(read.contracts);
  return isRefusal(v) ? "could not read contracts" : owned.length ? `holding ${owned.length} dwelling(s)` : "saving for a dwelling";
};

export const STRATEGIES: Record<string, Strategy> = {
  founder,
  "wage-maximiser": wageMaximiser,
  speculator,
  saver,
  slacker,
  borrower,
  landlord,
};
