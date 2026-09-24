// What a Jev player may do this hour, laid out by code (SJ.1). Each slot is one
// question: its candidates carry the tool call with the quantities and prices
// already worked out, the same arithmetic the scripted strategies use, and one
// sentence Jev reads as the option. Jev never sees an id, a quantity it would
// have to compute, or a price it would have to compare: the comparison is in
// the words. Every slot ends with a do-nothing option, because Jev always picks.

import * as act from "../../tools/act.ts";
import { currentJobs, jobOffers, lastPrice, median, workFullHours, type Script } from "../scripted/script.ts";

export type Candidate = {
  option: string;
  /** One sentence in the persona's own terms: what taking this option means. */
  describe: string;
  /** Null for a do-nothing option. Returns what happened, for the intent line. */
  act: ((s: Script) => Promise<string>) | null;
  /** Cannot be undone next hour (a switch, a founding, a purchase): held to a higher confidence. */
  irreversible: boolean;
};

export type Slot = {
  key: string;
  /** Execution order when several slots chose to act: hours first, then the job, then the persona's own business. */
  tier: number;
  /** "Decide only this: ..." the facts the choice turns on, in words, with the numbers precomputed. */
  ask: string;
  /** The same facts as data, for the state. */
  facts: Record<string, unknown>;
  candidates: Candidate[];
};

export type SlotGen = (s: Script) => Promise<Slot | null>;

const credits = (cents: number) => (cents / 100).toFixed(2);
const none = (option: string, describe: string): Candidate => ({ option, describe, act: null, irreversible: false });
const doing = (option: string, describe: string, act: (s: Script) => Promise<string>, irreversible = false): Candidate => ({ option, describe, act, irreversible });

/** How many hours, how hard: only when a contract exists to work. */
export const workSlot: SlotGen = async (s) => {
  const jobs = currentJobs(s.home);
  if (jobs.length === 0) return null;
  const budget = s.home.labor.budget;
  const contractHours = jobs.slice(0, 2).reduce((n, j) => n + j.maxHours, 0);
  const full = Math.max(1, Math.min(contractHours, budget));
  const half = Math.max(1, Math.floor(full / 2));
  const few = Math.max(1, Math.min(4, budget));
  const set = s.home.labor.allocations.reduce((n, a) => n + a.hours, 0);
  const best = Math.max(...jobs.map((j) => j.hourly));
  const fatigue = s.home.labor.fatigue_debt ?? 0;
  const run = (effort: "low" | "normal" | "high", cap: number) => async (sc: Script) => {
    const ok = await workFullHours(sc, effort, cap);
    return ok ? `set ${cap} h at ${effort} effort` : `could not set ${cap} h`;
  };
  return {
    key: "work",
    tier: 0,
    ask:
      `how many hours to work today and how hard. Your contracts allow ${contractHours} h a day at up to ${credits(best)} credits an hour; ` +
      `${budget} h of your day are still free and ${set} h are already set${fatigue ? `; you carry ${fatigue} h of fatigue debt` : ""}. ` +
      `Food ${s.food.toFixed(0)} of 100, balance ${credits(s.balance)} credits.`,
    facts: { contract_hours: contractHours, hour_budget: budget, hours_set_today: set, best_hourly_credits: credits(best), fatigue_debt: fatigue },
    candidates: [
      doing("full_normal", `work all ${full} contract hours at normal effort`, run("normal", full)),
      doing("full_high", `work all ${full} hours at high effort: more output, more fatigue`, run("high", full)),
      doing("half_normal", `work about half, ${half} hours, at normal effort`, run("normal", half)),
      doing("few_low", `work ${few} hours at low effort: the least that keeps a wage coming`, run("low", few)),
      none("none", "leave today's hours as they are already set"),
    ],
  };
};

/** Take a job when there is none; switch when an open offer pays more. */
export const jobSlot: SlotGen = async (s) => {
  const offers = jobOffers(await s.board());
  const jobs = currentJobs(s.home);
  const society = s.home.society;
  if (jobs.length === 0) {
    if (offers.length === 0) return null;
    const best = offers[0]!;
    const second = offers[1];
    const accept = (o: (typeof offers)[number]) => async (sc: Script) =>
      (await sc.do(act.acceptOffer, { offer: o.id })) ? `took job ${o.id} at ${credits(o.hourly)}/h` : `was refused job ${o.id}`;
    const candidates = [doing("take_best", `take the best-paying open job, ${credits(best.hourly)} an hour for up to ${best.maxHours} h a day`, accept(best))];
    if (second) candidates.push(doing("take_second", `take the next one instead, ${credits(second.hourly)} an hour for up to ${second.maxHours} h a day`, accept(second)));
    candidates.push(none("wait", "take nothing yet and wait for a better offer"));
    return {
      key: "job",
      tier: 1,
      ask:
        `whether to take a job now. You have none. ${offers.length} offer(s) are open; the best pays ${credits(best.hourly)} an hour` +
        `${second ? `, the next ${credits(second.hourly)}` : ""}. Food ${s.food.toFixed(0)} of 100, balance ${credits(s.balance)} credits; ` +
        `${society.unemployed} of ${society.population} citizens are unemployed.`,
      facts: { employed: false, offers_open: offers.length, best_offer_hourly: credits(best.hourly), second_offer_hourly: second ? credits(second.hourly) : null },
      candidates,
    };
  }
  const mine = Math.max(...jobs.map((j) => j.hourly));
  const best = offers.find((o) => !jobs.some((j) => j.org === o.org && j.workplace === o.workplace));
  if (!best || best.hourly <= mine) return null;
  const pct = Math.round((best.hourly / mine - 1) * 100);
  const worst = jobs.reduce((a, b) => (a.hourly <= b.hourly ? a : b));
  return {
    key: "job",
    tier: 1,
    ask:
      `whether to switch jobs. Yours pays ${credits(mine)} an hour; an open offer pays ${credits(best.hourly)}, ${pct}% more, for up to ${best.maxHours} h a day. ` +
      `Switching means taking it and giving notice on your current contract, which cannot be undone.`,
    facts: { employed: true, my_hourly: credits(mine), best_offer_hourly: credits(best.hourly), better_by_pct: pct },
    candidates: [
      doing(
        "switch",
        `take the offer at ${credits(best.hourly)} and give notice on the contract paying ${credits(worst.hourly)}`,
        async (sc) => {
          if (!(await sc.do(act.acceptOffer, { offer: best.id }))) return `was refused offer ${best.id}`;
          await sc.do(act.terminateContract, { contract: worst.contract });
          return `switched to offer ${best.id} at ${credits(best.hourly)}/h; notice on contract ${worst.contract}`;
        },
        true,
      ),
      none("stay", "keep the job you have"),
    ],
  };
};

const GOODS = ["grain", "ore", "materials", "wares", "machines"];

/** The speculator's book: buy under the trailing mean, sell over it, never more than a third of cash in stock. */
export const marketSlot: SlotGen = async (s) => {
  const books = await s.books();
  if (!books) return null;
  const means = (s.memory.means ??= {}) as Record<string, number[]>;
  const cashLimit = Math.floor(s.balance / 3);
  const held: Record<string, number> = {};
  for (const g of GOODS) held[g] = s.pantry[g] ?? 0;
  const stockValue = GOODS.reduce((sum, g) => sum + held[g]! * (lastPrice(books, g) ?? 0), 0);
  const facts: Record<string, unknown> = {};
  const lines: string[] = [];
  const candidates: Candidate[] = [];
  for (const g of GOODS) {
    const p = lastPrice(books, g);
    if (p === null) continue;
    const hist = (means[g] ??= []);
    hist.push(p);
    if (hist.length > 24) hist.shift();
    if (hist.length < 6) continue;
    const mean = hist.reduce((a, b) => a + b, 0) / hist.length;
    const pct = Math.round((p / mean - 1) * 100);
    facts[g] = { last: credits(p), vs_recent_mean_pct: pct, held: held[g] };
    lines.push(`${g} ${credits(p)} (${pct >= 0 ? "+" : ""}${pct}% against its mean${held[g] ? `, ${held[g]} held` : ""})`);
    if (p < mean * 0.9 && stockValue < cashLimit) {
      const qty = Math.max(1, Math.floor(Math.min(cashLimit - stockValue, s.balance / 4) / p));
      candidates.push(
        doing(`buy_${g}`, `bid for ${qty} ${g} at ${credits(p)}, ${-pct}% under its recent mean`, async (sc) =>
          (await sc.do(act.placeOrder, { instrument: g, side: "bid", qty, limit_price: p })) ? `bid ${qty} ${g} @${p}` : `bid for ${g} refused`,
        ),
      );
    } else if (p > mean * 1.1 && held[g]! > 0) {
      const qty = held[g]!;
      candidates.push(
        doing(`sell_${g}`, `ask ${qty} ${g} at ${credits(p)}, ${pct}% over its recent mean`, async (sc) =>
          (await sc.do(act.placeOrder, { instrument: g, side: "ask", qty, limit_price: p })) ? `asked ${qty} ${g} @${p}` : `ask for ${g} refused`,
        ),
      );
    }
  }
  if (candidates.length === 0) return null;
  candidates.push(none("hold", "place no order this hour"));
  return {
    key: "market",
    tier: 2,
    ask:
      `whether to trade. Prices against their recent means: ${lines.join("; ")}. ` +
      `You hold goods worth ${credits(stockValue)} against a limit of ${credits(cashLimit)}, a third of your ${credits(s.balance)} credits.`,
    facts,
    candidates,
  };
};

/** The founder's road: save, buy the Materials, found; then hire, sell the output, pay a dividend from surplus. */
export const ventureSlot: SlotGen = async (s) => {
  const orgs = await s.orgs();
  if (!orgs) return null;
  const books = await s.books();
  const mine = orgs.orgs.find((o) => o.i_manage);
  if (mine) {
    const offers = jobOffers(await s.board());
    const wage = median(offers.map((o) => o.hourly)) || 800;
    const wp = mine.workplaces[0];
    const workers = wp?.workers.length ?? 0;
    const inv = mine.inventory as Record<string, number>;
    const stock = Object.entries(inv).filter(([, q]) => q > 0);
    const sellable = stock.filter(([g]) => g !== "materials" && lastPrice(books, g) !== null);
    const candidates: Candidate[] = [];
    if (wp && workers < 2 && !s.memory.jobPosted) {
      candidates.push(
        doing("post_job", `post a job at ${credits(wage)} an hour, the going median, with two places`, async (sc) => {
          const ok = await sc.do(act.offerEmployment, { org: mine.id, workplace: wp.id, pay: { hourly: wage }, max_hours: 8, places: 2, notice_cycles: 1, term_cycles: null });
          if (ok) sc.memory.jobPosted = true;
          return ok ? `posted a job at ${credits(wage)}/h` : "job posting refused";
        }),
      );
    }
    if (sellable.length) {
      candidates.push(
        doing("sell_output", `ask ${sellable.map(([g, q]) => `${q} ${g}`).join(", ")} on the book at 5% over the last price`, async (sc) => {
          const done: string[] = [];
          for (const [g, q] of sellable) {
            if (sc.actionsLeft <= 0) break;
            const price = Math.round(lastPrice(books, g)! * 1.05);
            if (await sc.do(act.placeOrder, { instrument: g, side: "ask", qty: q, limit_price: price, on_behalf_of: mine.id })) done.push(`asked ${q} ${g} @${price}`);
          }
          return done.length ? done.join(", ") : "asks refused";
        }),
      );
    }
    const perShare = Math.floor(mine.treasury / 4 / Math.max(1, mine.my_shares));
    if (mine.treasury > 3 * wage * 8 * 2 && perShare > 0) {
      candidates.push(
        doing("pay_dividend", `declare a dividend of ${credits(perShare)} a share, a quarter of a treasury of ${credits(mine.treasury)}`, async (sc) =>
          (await sc.do(act.dividend, { org: mine.id, per_share: perShare })) ? `declared ${credits(perShare)} a share` : "dividend refused",
        ),
      );
    }
    if (candidates.length === 0) return null;
    candidates.push(none("run_quietly", "change nothing at the firm this hour"));
    return {
      key: "venture",
      tier: 2,
      ask:
        `what to do at the firm you manage. Treasury ${credits(mine.treasury)} credits; ${workers} worker(s) at its first workplace; ` +
        `inventory ${stock.length ? stock.map(([g, q]) => `${q} ${g}`).join(", ") : "empty"}; the going wage is ${credits(wage)} an hour.`,
      facts: { managing: true, treasury: credits(mine.treasury), workers, inventory: Object.fromEntries(stock), going_wage: credits(wage) },
      candidates,
    };
  }
  const need = orgs.founding.materials;
  const fee = orgs.founding.money;
  const have = s.pantry.materials ?? 0;
  const pm = lastPrice(books, "materials");
  const candidates: Candidate[] = [];
  if (have >= need && s.balance >= fee) {
    candidates.push(
      doing(
        "found_now",
        `found a firm now, paying the ${credits(fee)} fee and the ${need} Materials, and open a mine`,
        async (sc) => {
          const name = `${sc.handle}'s Works`;
          return (await sc.do(act.foundOrg, { kind: "firm", name, first_workplace: { kind: "mine", slot: null } })) ? `founded ${name} with a mine` : "founding refused";
        },
        true,
      ),
    );
  }
  if (have < need && pm !== null && s.balance > fee + pm * (need - have)) {
    const qty = need - have;
    const price = Math.round(pm * 1.1);
    candidates.push(
      doing("buy_materials", `bid for the ${qty} Materials still needed at ${credits(price)} each, 10% over the last price, keeping the fee in hand`, async (sc) =>
        (await sc.do(act.placeOrder, { instrument: "materials", side: "bid", qty, limit_price: price })) ? `bid ${qty} materials @${price}` : "materials bid refused",
      ),
    );
  }
  if (candidates.length === 0) return null;
  candidates.push(none("keep_saving", "keep saving; found nothing and buy nothing this hour"));
  return {
    key: "venture",
    tier: 2,
    ask:
      `whether to move toward founding a firm. It costs a ${credits(fee)} fee and ${need} Materials; you hold ${have} Materials and ${credits(s.balance)} credits` +
      `${pm !== null ? `; Materials last traded at ${credits(pm)}` : ""}.`,
    facts: { managing: false, fee: credits(fee), materials_need: need, materials_have: have, materials_last: pm !== null ? credits(pm) : null },
    candidates,
  };
};

/** The landlord's ledger: buy a dwelling on sale when affordable, offer an owned one to let. */
export const propertySlot: SlotGen = async (s) => {
  const board = await s.board();
  const owned = (s.memory.dwellings ??= []) as number[];
  const sales = board
    .filter((o) => o.kind === "sale")
    .map((o) => {
      const sale = (o.body as { sale?: { asset?: { dwelling?: number }; price?: { money?: number } } }).sale;
      return { id: o.id, dwelling: sale?.asset?.dwelling, price: sale?.price?.money };
    })
    .filter((x): x is { id: number; dwelling: number; price: number } => x.dwelling !== undefined && x.price !== undefined)
    .sort((a, b) => a.price - b.price);
  const cheapest = sales.find((x) => x.price <= s.balance * 0.8);
  const wages = jobOffers(board).map((j) => j.hourly);
  const dayWage = (median(wages) || 800) * 8;
  const rent = Math.max(100, Math.round(dayWage * 0.2));
  const unleased = owned.filter((d) => !s.memory[`leased:${d}`]);
  const candidates: Candidate[] = [];
  if (cheapest) {
    candidates.push(
      doing(
        "buy_dwelling",
        `buy the cheapest dwelling on sale for ${credits(cheapest.price)}, ${Math.round((cheapest.price / Math.max(1, s.balance)) * 100)}% of your money`,
        async (sc) => {
          if (await sc.do(act.acceptOffer, { offer: cheapest.id })) {
            owned.push(cheapest.dwelling);
            return `bought dwelling ${cheapest.dwelling} for ${credits(cheapest.price)}`;
          }
          return "purchase refused";
        },
        true,
      ),
    );
  }
  const d = unleased[0];
  if (d !== undefined) {
    candidates.push(
      doing("offer_lease", `offer one of your empty dwellings to let at ${credits(rent)} a day, a fifth of a day's median wage`, async (sc) => {
        if (await sc.do(act.offerLease, { asset: { dwelling: d }, rent_per_cycle: rent, term_cycles: null, on_behalf_of: null })) {
          sc.memory[`leased:${d}`] = true;
          return `offered dwelling ${d} at ${credits(rent)} a day`;
        }
        return "lease offer refused";
      }),
    );
  }
  if (candidates.length === 0) return null;
  candidates.push(none("hold", "buy nothing and post nothing this hour"));
  return {
    key: "property",
    tier: 2,
    ask:
      `what to do about property. ${sales.length} dwelling(s) are on sale${cheapest ? `, the cheapest within reach at ${credits(cheapest.price)}` : ", none within four fifths of your money"}; ` +
      `you own ${owned.length}, ${unleased.length} not yet offered to let. Balance ${credits(s.balance)} credits; a day's median wage is about ${credits(dayWage)}.`,
    facts: { for_sale: sales.length, cheapest_affordable: cheapest ? credits(cheapest.price) : null, owned: owned.length, unleased: unleased.length, rent_would_be: credits(rent) },
    candidates,
  };
};

/** The slots a persona is asked about, in tier order; a slug without its own list lives like a householder. */
export const SLOTS: Record<string, SlotGen[]> = {
  founder: [workSlot, jobSlot, ventureSlot],
  "wage-maximiser": [workSlot, jobSlot],
  speculator: [workSlot, jobSlot, marketSlot],
  landlord: [workSlot, jobSlot, propertySlot],
};

export function slotsFor(slug: string): SlotGen[] {
  return SLOTS[slug] ?? [workSlot, jobSlot];
}

/** Every slot that has a real choice in it this hour. */
export async function layOut(slug: string, s: Script): Promise<Slot[]> {
  const out: Slot[] = [];
  for (const gen of slotsFor(slug)) {
    const slot = await gen(s);
    if (slot) out.push(slot);
  }
  return out;
}
