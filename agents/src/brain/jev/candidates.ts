// What a Jev player may do this hour, laid out by code (SJ.1, SJ.2). Each slot
// is one question: its candidates carry the tool call with the quantities and
// prices already worked out, the same arithmetic the scripted strategies use,
// and one sentence Jev reads as the option. Jev never sees an id, a quantity
// it would have to compute, or a price it would have to compare: the comparison
// is in the words. Every slot ends with a do-nothing option, because Jev always
// picks; and a slot is offered only when there is a reason to choose, so that
// the hours are not re-asked every hour they are already set (SJ.2).

import * as act from "../../tools/act.ts";
import { currentJobs, goingRent, jobOffers, lastPrice, median, workFullHours, workerContracts, type BooksView, type Offer, type Script } from "../scripted/script.ts";
import { dwellingAskPrice } from "../scripted/strategies.ts";

export type Candidate = {
  option: string;
  /** One sentence in the persona's own terms: what taking this option means. */
  describe: string;
  /** Null for a do-nothing option. Returns what happened, for the intent line. */
  act: ((s: Script) => Promise<string>) | null;
  /** Cannot be undone next hour (a switch, a founding, a purchase, a loan): held to a higher confidence. */
  irreversible: boolean;
};

export type Slot = {
  key: string;
  /** Execution order when several slots chose to act: hours, then a roof and a job, then the persona's own business, then the plan. */
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

/** The market's tick (SJ.5, Q163): prices are whole cents, so the smallest undercut is one. */
export const TICK = 1;

/** One good's book as a price-setter reads it (SJ.5): the last print, the best resting bid and ask, and how deep each side is. */
export type Book = { last: number | null; bestBid: number | null; bestAsk: number | null; bidDepth: number; askDepth: number };

export function bookOf(books: BooksView | null, instrument: string): Book {
  const b = books?.books.find((x) => x.instrument === instrument);
  return { last: b?.last_price ?? null, bestBid: b?.best_bid ?? null, bestAsk: b?.best_ask ?? null, bidDepth: b?.bid_depth ?? 0, askDepth: b?.ask_depth ?? 0 };
}

/** One tick under the best ask, or under the last print when nobody is asking: the price that sells first. */
export function undercutPrice(b: Book): number | null {
  const ref = b.bestAsk ?? b.last;
  return ref === null ? null : Math.max(TICK, ref - TICK);
}

/** The book's numbers as facts for the state, with the days the firm's shelf has not shrunk. */
function bookFacts(b: Book, daysUnsold?: number): Record<string, unknown> {
  return {
    last: b.last !== null ? credits(b.last) : null,
    best_bid: b.bestBid !== null ? credits(b.bestBid) : null,
    best_ask: b.bestAsk !== null ? credits(b.bestAsk) : null,
    bid_depth: b.bidDepth,
    ask_depth: b.askDepth,
    ...(daysUnsold !== undefined ? { days_unsold: daysUnsold } : {}),
  };
}

/** How many days each good on the firm's shelf has gone without shrinking (SJ.5): the shelf is read once a day and compared with the day before. */
function daysUnsold(s: Script, inventory: Record<string, number>): Record<string, number> {
  const shelf = (s.memory.shelf ??= {}) as Record<string, { cycle: number; qty: number }>;
  const unsold = (s.memory.unsold ??= {}) as Record<string, number>;
  for (const [good, qty] of Object.entries(inventory)) {
    const seen = shelf[good];
    if (seen && seen.cycle === s.clock.cycle) continue;
    if (seen) unsold[good] = qty >= seen.qty ? (unsold[good] ?? 0) + (s.clock.cycle - seen.cycle) : 0;
    else unsold[good] = 0;
    shelf[good] = { cycle: s.clock.cycle, qty };
  }
  for (const good of Object.keys(unsold)) if (!(good in inventory)) delete unsold[good];
  return unsold;
}

/** What a day's work pays at this citizen's best rate, or the board's median, or the legacy wage. */
function dayWage(s: Script, offers: ReturnType<typeof jobOffers>): number {
  const jobs = currentJobs(s.home);
  const hourly = jobs.length ? Math.max(...jobs.map((j) => j.hourly)) : median(offers.map((o) => o.hourly)) || 800;
  return hourly * 8;
}

/** The firm a citizen manages, as the firm slot last saw it (SJ.3): kept in memory so `work` can offer the manager's own workplace without a second read of the orgs. */
export type Firm = {
  org: number;
  workplace: number;
  kind: string;
  produces: string;
  /** Units a worker-hour before skill, effort and needs. */
  base_rate: number;
  /** Inputs a unit consumes and how many the firm holds: production is capped by them. */
  consumes: Record<string, number>;
  inventory: Record<string, number>;
  /** The last price of what it makes, in cents, or null when nothing has traded. */
  price: number | null;
};

/** The token wage a manager pays itself to hold a place at its own workplace (Q160): the smallest the engine accepts; the firm's output is the return. */
export const OWN_PLACE_WAGE = 1;

/** Hire yourself (Q160) if you hold no place at your own workplace, then put the hours there. */
function workOwn(firm: Firm, hours: number) {
  return async (sc: Script) => {
    if (!currentJobs(sc.home).some((j) => j.workplace === firm.workplace)) {
      const mine = (offers: ReturnType<typeof jobOffers>) => offers.find((o) => o.org === firm.org && o.workplace === firm.workplace && o.hourly <= OWN_PLACE_WAGE);
      let place = mine(jobOffers(await sc.board()));
      if (!place) {
        const posted = await sc.do(act.offerEmployment, { org: firm.org, workplace: firm.workplace, pay: { hourly: OWN_PLACE_WAGE }, max_hours: hours, places: 1, notice_cycles: 0, term_cycles: null });
        if (!posted) return `could not post my own place at the ${firm.kind}`;
        place = mine(jobOffers(await sc.board()));
        if (!place) return "posted my own place but it is not on the board";
      }
      if (!(await sc.do(act.acceptOffer, { offer: place.id }))) return `was refused my own place at the ${firm.kind}`;
    }
    const ok = await sc.do(act.setLabor, { allocations: [{ workplace: firm.workplace, hours, effort: "normal" }] });
    return ok ? `set ${hours} h at my own ${firm.kind}` : `could not set ${hours} h at my own ${firm.kind}`;
  };
}

/** What a day of one citizen's hours makes at `firm`, capped by the inputs it holds. */
export function ownDayOutput(firm: Firm, hours: number, outputMult: number): number {
  let units = Math.floor(firm.base_rate * hours * outputMult);
  for (const [good, per] of Object.entries(firm.consumes)) if (per > 0) units = Math.min(units, Math.floor((firm.inventory[good] ?? 0) / per));
  return Math.max(0, units);
}

/** How many hours, how hard. The full menu when nothing is set today; afterwards only when something gives a reason to change. */
export const workSlot: SlotGen = async (s) => {
  const jobs = currentJobs(s.home);
  if (jobs.length === 0) return null;
  const budget = s.home.labor.budget;
  const contractHours = jobs.slice(0, 2).reduce((n, j) => n + j.maxHours, 0);
  const full = Math.max(1, Math.min(contractHours, budget));
  const half = Math.max(1, Math.floor(full / 2));
  const few = Math.max(1, Math.min(4, budget));
  const set = s.home.labor.allocations.reduce((n, a) => n + a.hours, 0);
  const effort = s.home.labor.allocations[0]?.effort ?? "normal";
  // The manager's own workplace (SJ.3), when the firm slot has seen one and a wage job exists beside it.
  const firm = s.memory.firm as Firm | undefined;
  const ownHours = firm ? (s.home.labor.allocations.find((a) => a.workplace === firm.workplace)?.hours ?? 0) : 0;
  const wageJobs = firm ? jobs.filter((j) => j.workplace !== firm.workplace) : jobs;
  const best = wageJobs.length ? Math.max(...wageJobs.map((j) => j.hourly)) : 0;
  const fatigue = s.home.labor.fatigue_debt ?? 0;
  const day = best * 8;
  const run = (e: "low" | "normal" | "high", cap: number) => async (sc: Script) => {
    // Once the hours are at the manager's own workplace, a change keeps them there.
    const ok = firm && ownHours > 0 ? await sc.do(act.setLabor, { allocations: [{ workplace: firm.workplace, hours: cap, effort: e }] }) : await workFullHours(sc, e, cap);
    return ok ? `set ${cap} h at ${e} effort` : `could not set ${cap} h`;
  };
  const facts: Record<string, unknown> = { contract_hours: contractHours, hour_budget: budget, hours_set_today: set, effort_set: effort, best_hourly_credits: credits(best), fatigue_debt: fatigue };
  const who = `Food ${s.food.toFixed(0)} of 100, balance ${credits(s.balance)} credits, a day's wage ${credits(day)}.`;
  const own: Candidate[] = [];
  const ownReasons: string[] = [];
  if (firm && wageJobs.length && ownHours < full && onceToday(s, "work_own")) {
    const units = ownDayOutput(firm, full, s.home.labor.output_mult ?? 1);
    const value = firm.price !== null ? units * firm.price : null;
    facts.own_workplace = { kind: firm.kind, hours_there: ownHours, day_output_units: units, day_output_value_credits: value !== null ? credits(value) : null };
    ownReasons.push(`your own ${firm.kind} stands without your hours`);
    own.push(
      doing(
        "work_own",
        `work all ${full} hours at your own ${firm.kind} for a token wage: about ${units} ${firm.produces} a day at your rate${value !== null ? `, worth ${credits(value)} at the last price` : ""}, the firm's to sell, instead of the ${credits(day)} a day your job pays`,
        workOwn(firm, full),
      ),
    );
  } else if (firm && wageJobs.length && ownHours >= full && onceToday(s, "work_job")) {
    facts.own_workplace = { kind: firm.kind, hours_there: ownHours };
    ownReasons.push(`your hours are at your own ${firm.kind} and your job at ${credits(best)} an hour goes unworked`);
    own.push(doing("work_job", `work all ${full} hours at the job paying ${credits(day)} a day and leave the ${firm.kind} to its hands`, async (sc) => ((await workFullHours(sc, "normal", full)) ? `set ${full} h at the job` : "could not set the hours")));
  }
  if (set === 0) {
    return {
      key: "work",
      tier: 0,
      ask:
        `how many hours to work today and how hard. Your contracts allow ${contractHours} h a day at up to ${credits(best)} credits an hour; ` +
        `${budget} h of your day are free and nothing is set yet${fatigue ? `; you carry ${fatigue} h of fatigue debt` : ""}${ownReasons.length ? `; ${ownReasons.join(", ")}` : ""}. ${who}`,
      facts,
      candidates: [
        doing("full_normal", `work all ${full} contract hours at normal effort`, run("normal", full)),
        doing("full_high", `work all ${full} hours at high effort: more output, more fatigue`, run("high", full)),
        doing("half_normal", `work about half, ${half} hours, at normal effort`, run("normal", half)),
        doing("few_low", `work ${few} hours at low effort: the least that keeps a wage coming`, run("low", few)),
        ...own,
        none("none", "set no hours today"),
      ],
    };
  }
  const reasons: string[] = [...ownReasons];
  const candidates: Candidate[] = [...own];
  if (fatigue > 0 && set > half) {
    reasons.push(`you carry ${fatigue} h of fatigue debt`);
    candidates.push(doing("rest_more", `cut today to ${half} hours at normal effort and work the fatigue off`, run("normal", half)));
  }
  if (s.food < 40 && set < full) {
    reasons.push(`Food is down to ${s.food.toFixed(0)}`);
    candidates.push(doing("work_more", `work all ${full} contract hours at normal effort`, run("normal", full)));
  }
  if (s.balance < day && effort !== "high" && fatigue === 0) {
    reasons.push(`your balance is under a day's wage`);
    candidates.push(doing("push_harder", `work all ${full} hours at high effort for the extra output, at the cost of fatigue`, run("high", full)));
  }
  if (s.food >= 80 && s.balance > 5 * day && set >= full && effort !== "low") {
    reasons.push(`you have ${credits(s.balance)} credits and Food is ${s.food.toFixed(0)}`);
    candidates.push(doing("ease_off", `drop to ${few} hours at low effort today`, run("low", few)));
  }
  if (candidates.length === 0) return null;
  candidates.push(none("keep_hours", `keep the ${set} hours at ${effort} effort already set`));
  return {
    key: "work",
    tier: 0,
    ask: `whether to change today's hours. ${set} h at ${effort} effort are set of the ${contractHours} your contracts allow, and ${reasons.join(", ")}. ${who}`,
    facts,
    candidates,
  };
};

/** A roof: rent the cheapest dwelling to let while unhoused. Every persona gets this one. */
export const housingSlot: SlotGen = async (s) => {
  if (s.home.household.dwelling) return null;
  const board = await s.board();
  const leases = board
    .filter((o) => o.kind === "lease")
    .map((o) => {
      const l = (o.body as { lease?: { asset?: { dwelling?: number }; rent_per_cycle?: number } }).lease;
      return { id: o.id, dwelling: l?.asset?.dwelling, rent: l?.rent_per_cycle };
    })
    .filter((x): x is { id: number; dwelling: number; rent: number } => x.dwelling !== undefined && x.rent !== undefined)
    .sort((a, b) => a.rent - b.rent);
  const cheapest = leases[0];
  if (!cheapest) return null;
  const day = dayWage(s, jobOffers(board));
  const pct = Math.round((cheapest.rent / Math.max(1, day)) * 100);
  return {
    key: "housing",
    tier: 1,
    ask:
      `whether to rent a home. You are unhoused: Shelter is ${s.home.needs.shelter.toFixed(0)} of 100 and falls 2 an hour without a roof, and unhoused hands produce 30% less. ` +
      `The cheapest dwelling to let costs ${credits(cheapest.rent)} a day, ${pct}% of a day's wage at your rate; ${leases.length} are offered. Balance ${credits(s.balance)} credits.`,
    facts: { housed: false, shelter: Math.round(s.home.needs.shelter), cheapest_rent: credits(cheapest.rent), rent_pct_of_day_wage: pct, dwellings_to_let: leases.length },
    candidates: [
      doing("rent_cheapest", `rent the cheapest dwelling at ${credits(cheapest.rent)} a day and move in`, async (sc) =>
        (await sc.do(act.acceptOffer, { offer: cheapest.id })) ? `rented dwelling ${cheapest.dwelling} at ${credits(cheapest.rent)} a day` : "the lease was refused",
      ),
      none("stay_unhoused", "stay without a roof and keep the money"),
    ],
  };
};

/** Take a job when there is none; switch when an open offer pays more. */
export const jobSlot: SlotGen = async (s) => {
  const offers = jobOffers(await s.board());
  const jobs = currentJobs(s.home);
  const society = s.home.society;
  if (jobs.length === 0) {
    // The days without a job (SJ.5), so waiting is re-asked as a change with a cost and not a resting state.
    if (typeof s.memory.unemployedSince !== "number") s.memory.unemployedSince = s.clock.cycle;
    const idle = s.clock.cycle - (s.memory.unemployedSince as number);
    if (offers.length === 0) return null;
    const best = offers[0]!;
    const second = offers[1];
    const accept = (o: (typeof offers)[number]) => async (sc: Script) =>
      (await sc.do(act.acceptOffer, { offer: o.id })) ? `took job ${o.id} at ${credits(o.hourly)}/h` : `was refused job ${o.id}`;
    const candidates = [doing("take_best", `take the best-paying open job, ${credits(best.hourly)} an hour for up to ${best.maxHours} h a day`, accept(best))];
    if (second) candidates.push(doing("take_second", `take the next one instead, ${credits(second.hourly)} an hour for up to ${second.maxHours} h a day`, accept(second)));
    candidates.push(none("wait", idle > 0 ? "keep waiting, another day without a wage" : "take nothing yet and wait for a better offer"));
    const unearned = idle * best.hourly * 8;
    return {
      key: "job",
      tier: 1,
      ask:
        `whether to take a job now. You have none. ${offers.length} offer(s) are open; the best pays ${credits(best.hourly)} an hour` +
        `${second ? `, the next ${credits(second.hourly)}` : ""}. Food ${s.food.toFixed(0)} of 100, balance ${credits(s.balance)} credits; ` +
        `${society.unemployed} of ${society.population} citizens are unemployed.` +
        (idle > 0 ? ` You have had no job for ${idle} day(s) and ${credits(unearned)}, ${idle} day(s) of the best wage, has gone unearned.` : ""),
      facts: {
        employed: false,
        offers_open: offers.length,
        best_offer_hourly: credits(best.hourly),
        second_offer_hourly: second ? credits(second.hourly) : null,
        ...(idle > 0 ? { days_without_a_job: idle, unearned_credits: credits(unearned) } : {}),
      },
      candidates,
    };
  }
  delete s.memory.unemployedSince;
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

/** The standing plan as `/home` carries it, reduced to what `set_plan` takes; unknown fields fall to the tool's defaults. */
function planOf(s: Script) {
  const p = (s.home.plan ?? {}) as Record<string, unknown>;
  const labor = ["explicit", "accept_assignment", "follow_norm"].includes(String(p.labor)) ? (p.labor as "explicit") : "explicit";
  return {
    labor,
    keep_food_at_least: typeof p.keep_food_at_least === "number" ? p.keep_food_at_least : 24,
    max_food_price: typeof p.max_food_price === "number" ? p.max_food_price : null,
    buy_wares_when: null,
    keep_balance_at_least: typeof p.keep_balance_at_least === "number" ? p.keep_balance_at_least : 0,
    standing_orders: [],
  };
}

/** True once a day for `key`: a question worth asking daily, not hourly (a declined change is not re-asked until tomorrow). */
function onceToday(s: Script, key: string): boolean {
  const k = `asked:${key}`;
  if (s.memory[k] === s.clock.cycle) return false;
  s.memory[k] = s.clock.cycle;
  return true;
}

/** The standing plan's two floors, when Food or the balance gives a reason to move one. Asked once a day, except that low Food asks at once. */
export const planSlot: SlotGen = async (s) => {
  const plan = planOf(s);
  const urgent = s.food < 45;
  if (!urgent && !onceToday(s, "plan")) return null;
  const day = dayWage(s, jobOffers(await s.board()));
  const candidates: Candidate[] = [];
  const reasons: string[] = [];
  const set = (patch: Partial<typeof plan>, label: string) => async (sc: Script) => ((await sc.do(act.setPlan, { ...plan, ...patch })) ? label : "the plan was refused");
  if (s.food < 45 && plan.keep_food_at_least < 48) {
    reasons.push(`Food is ${s.food.toFixed(0)} and the plan keeps only ${plan.keep_food_at_least} Food in the pantry`);
    candidates.push(doing("stock_more_food", "have the plan keep 48 Food in the pantry, two days' worth, buying as needed", set({ keep_food_at_least: 48 }, "the plan keeps 48 Food now")));
  }
  if (s.balance > 6 * day && plan.keep_balance_at_least < s.balance * 0.5) {
    const floor = Math.floor(s.balance * 0.6);
    reasons.push(`you hold ${credits(s.balance)} credits, more than six days' wages, and the plan protects ${credits(plan.keep_balance_at_least)}`);
    candidates.push(doing("put_aside", `have the plan keep ${credits(floor)} credits untouchable, three fifths of what you hold`, set({ keep_balance_at_least: floor }, `the plan protects ${credits(floor)} now`)));
  }
  if (plan.keep_balance_at_least > s.balance * 0.9 && s.food < 40) {
    const floor = Math.floor(s.balance * 0.3);
    reasons.push(`the plan protects ${credits(plan.keep_balance_at_least)} of your ${credits(s.balance)} and Food is ${s.food.toFixed(0)}`);
    candidates.push(doing("loosen_floor", `lower the protected balance to ${credits(floor)} so the plan can buy Food`, set({ keep_balance_at_least: floor }, `the plan protects ${credits(floor)} now`)));
  }
  if (candidates.length === 0) return null;
  candidates.push(none("keep_plan", "leave the standing plan as it is"));
  return {
    key: "plan",
    tier: 3,
    ask: `whether to change your standing plan, which buys for you every hour: ${reasons.join("; ")}.`,
    facts: { keep_food_at_least: plan.keep_food_at_least, keep_balance_at_least: credits(plan.keep_balance_at_least), day_wage: credits(day) },
    candidates,
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

/** The borrower's loan: the cheapest credit on the board, once. */
export const creditSlot: SlotGen = async (s) => {
  if (s.memory.borrowed) return null;
  const offers = (await s.board())
    .filter((o) => o.kind === "credit")
    .map((o) => {
      const c = (o.body as { credit?: { principal?: number; rate_per_cycle_bp?: number; term_cycles?: number } }).credit;
      return { id: o.id, principal: c?.principal, bp: c?.rate_per_cycle_bp, term: c?.term_cycles };
    })
    .filter((x): x is { id: number; principal: number; bp: number; term: number } => x.principal !== undefined && x.bp !== undefined && x.term !== undefined)
    .sort((a, b) => a.bp - b.bp);
  const best = offers[0];
  if (!best) return null;
  const installment = Math.round((best.principal * (1 + (best.bp / 10000) * best.term)) / best.term);
  const day = dayWage(s, jobOffers(await s.board()));
  return {
    key: "credit",
    tier: 2,
    ask:
      `whether to borrow. The cheapest loan offered is ${credits(best.principal)} credits at ${(best.bp / 100).toFixed(2)}% a day over ${best.term} days: ` +
      `about ${credits(installment)} a day to repay, ${Math.round((installment / Math.max(1, day)) * 100)}% of a day's wage. A missed installment seizes collateral and then flags default. ` +
      `You hold ${credits(s.balance)} credits.`,
    facts: { principal: credits(best.principal), rate_pct_per_day: (best.bp / 100).toFixed(2), term_days: best.term, installment: credits(installment), installment_pct_of_day_wage: Math.round((installment / Math.max(1, day)) * 100) },
    candidates: [
      doing(
        "take_credit",
        `take the ${credits(best.principal)} loan and repay ${credits(installment)} a day for ${best.term} days`,
        async (sc) => {
          if (await sc.do(act.acceptOffer, { offer: best.id })) {
            sc.memory.borrowed = true;
            return `took credit offer ${best.id} for ${credits(best.principal)}`;
          }
          return "the loan was refused";
        },
        true,
      ),
      none("no_credit", "borrow nothing"),
    ],
  };
};

/** The workplaces a founder may open on the strength of a margin (SJ.5): a Builder's output is a dwelling, priced by the `dwelling` slot, so it is not among them. */
export const WORKPLACE_KINDS = ["mine", "mill", "workshop", "foundry"] as const;
export type WorkplaceKind = (typeof WORKPLACE_KINDS)[number] | "builder";

/** What a day at one workplace of `kind` clears (SJ.5): the product's last price × the base rate × 8 h, less the inputs at their last price and a day's wage; null when a price is missing. */
export function dayMargin(kind: string, orgs: { recipes: { workplace_kind: string; produces: string; consumes: Record<string, number>; base_rate: number }[] }, books: BooksView | null, dayWage: number): { units: number; revenue: number; inputs: number; margin: number } | null {
  const recipe = orgs.recipes.find((r) => r.workplace_kind === kind);
  if (!recipe) return null;
  const price = lastPrice(books, recipe.produces);
  if (price === null) return null;
  const units = Math.floor(recipe.base_rate * 8);
  let inputs = 0;
  for (const [good, per] of Object.entries(recipe.consumes)) {
    const p = lastPrice(books, good);
    if (p === null) return null;
    inputs += per * units * p;
  }
  const revenue = units * price;
  return { units, revenue, inputs, margin: revenue - inputs - dayWage };
}

/** Which workplace to open (SJ.5): once a day when the Materials and the fee are in hand, the four kinds ranked by a day's margin at the last prices, the best first. */
export const workplaceSlot: SlotGen = async (s) => {
  // No Materials, nothing to open: spare the read.
  if ((s.pantry.materials ?? 0) === 0) return null;
  const orgs = await s.orgs();
  if (!orgs || orgs.orgs.some((o) => o.i_manage)) return null;
  if ((s.pantry.materials ?? 0) < orgs.founding.materials || s.balance < orgs.founding.money) return null;
  if (!onceToday(s, "which_workplace")) return null;
  const books = await s.books();
  const wage = dayWage(s, jobOffers(await s.board()));
  const ranked = WORKPLACE_KINDS.map((kind) => ({ kind, m: dayMargin(kind, orgs, books, wage) }))
    .filter((x): x is { kind: (typeof WORKPLACE_KINDS)[number]; m: NonNullable<ReturnType<typeof dayMargin>> } => x.m !== null)
    .sort((a, b) => b.m.margin - a.m.margin);
  if (ranked.length === 0) return null;
  const recipeOf = (kind: string) => orgs.recipes.find((r) => r.workplace_kind === kind)!;
  const facts: Record<string, unknown> = { day_wage: credits(wage) };
  for (const { kind, m } of ranked) facts[kind] = { units_a_day: m.units, revenue: credits(m.revenue), inputs: credits(m.inputs), margin: credits(m.margin) };
  return {
    key: "which_workplace",
    tier: 2,
    ask:
      `which workplace to open, now that you hold the ${orgs.founding.materials} Materials and the ${credits(orgs.founding.money)} fee. One worker's day at each, at the last prices and a day's wage of ${credits(wage)}: ` +
      ranked.map(({ kind, m }) => `${kind} ${m.margin >= 0 ? "clears" : "loses"} ${credits(Math.abs(m.margin))}`).join(", ") +
      ". Founding follows tomorrow's question, not this one.",
    facts,
    candidates: [
      ...ranked.map(({ kind, m }) => {
        const r = recipeOf(kind);
        const inputs = Object.entries(r.consumes).map(([g, per]) => `${per * m.units} ${g}`).join(", ");
        return doing(
          `open_${kind}`,
          `open a ${kind}: about ${m.units} ${r.produces} a day, worth ${credits(m.revenue)}${inputs ? `, less ${credits(m.inputs)} for ${inputs}` : ""} and ${credits(wage)} in wages, ${m.margin >= 0 ? "clearing" : "losing"} ${credits(Math.abs(m.margin))} a day`,
          async (sc) => {
            sc.memory.workplaceKind = kind;
            return `chose a ${kind}`;
          },
        );
      }),
      none("decide_later", "open nothing yet; decide another day"),
    ],
  };
};

/** The founder's road: save, buy the Materials, found a firm of `kind` (or, for a chooser, the kind `which_workplace` picked); then fund it, hire at one of two wages, keep it in inputs, sell the output, lay off when wages outrun output, pay a dividend from surplus. */
export function ventureSlotOf(kind: WorkplaceKind, firmName: string, choose = false): SlotGen {
  return async (s) => {
    const orgs = await s.orgs();
    if (!orgs) return null;
    const books = await s.books();
    const mine = orgs.orgs.find((o) => o.i_manage);
    if (mine) {
      const offers = jobOffers(await s.board());
      const wage = median(offers.map((o) => o.hourly)) || 800;
      const generous = Math.round(wage * 1.15);
      const wp = mine.workplaces[0];
      const recipe = orgs.recipes.find((r) => r.workplace_kind === wp?.kind);
      const produces = recipe?.produces ?? "output";
      const price = lastPrice(books, produces);
      const inv = mine.inventory as Record<string, number>;
      // What `work` needs to offer the manager's own workplace next hour (SJ.3), without a second read.
      if (wp) {
        const firm: Firm = { org: mine.id, workplace: wp.id, kind: wp.kind, produces, base_rate: recipe?.base_rate ?? 0, consumes: (recipe?.consumes ?? {}) as Record<string, number>, inventory: inv, price };
        s.memory.firm = firm;
      }
      // The hired hands, their cost against yesterday's output (SJ.3): the manager's own hours are not a wage.
      const hands = wp?.workers.filter((w) => w.citizen !== s.me) ?? [];
      const workers = hands.length;
      const wagesDay = workers * wage * 8;
      const yesterdayUnits = Math.round(wp?.last_cycle_output ?? 0);
      const yesterdayValue = price !== null ? yesterdayUnits * price : null;
      const stock = Object.entries(inv).filter(([, q]) => q > 0);
      const consumes = (recipe?.consumes ?? {}) as Record<string, number>;
      const sellable = stock.filter(([g]) => !(g in consumes) && g !== "materials" && lastPrice(books, g) !== null);
      const dayOfWages = wage * 8;
      const coversDays = Math.floor(mine.treasury / Math.max(1, dayOfWages));
      const candidates: Candidate[] = [];
      const postJob = (hourly: number, label: string) => async (sc: Script) => {
        const ok = await sc.do(act.offerEmployment, { org: mine.id, workplace: wp!.id, pay: { hourly }, max_hours: 8, places: 2, notice_cycles: 1, term_cycles: null });
        if (ok) {
          sc.memory.jobPosted = true;
          sc.memory.hiredCycle = sc.clock.cycle;
        }
        return ok ? `posted a job at ${credits(hourly)}/h (${label})` : "job posting refused";
      };
      if (wp && workers < 2 && !s.memory.jobPosted) {
        candidates.push(doing("post_job", `post a job at ${credits(wage)} an hour, the going median, with two places`, postJob(wage, "median")));
        candidates.push(doing("post_job_generous", `post a job at ${credits(generous)} an hour, 15% over the median, to draw workers away from other firms`, postJob(generous, "generous")));
      }
      // A full day of the hands' work has been seen, and it sold for less than it cost.
      const hired = typeof s.memory.hiredCycle === "number" ? (s.memory.hiredCycle as number) : s.clock.cycle;
      if (workers > 0 && yesterdayValue !== null && wagesDay > yesterdayValue && s.clock.cycle >= hired + 2) {
        candidates.push(
          doing("lay_off", `lay off one of the ${workers} hired hand(s), paying a day's notice, since their ${credits(wagesDay)} a day in wages outran yesterday's ${credits(yesterdayValue)} of output`, async (sc) => {
            const held = workerContracts(await sc.contracts(), mine.id).filter((k) => k.citizen !== sc.me);
            const dearest = held.sort((a, b) => b.hourly - a.hourly)[0];
            if (!dearest) return "found no contract to end";
            if (await sc.do(act.terminateContract, { contract: dearest.contract, on_behalf_of: mine.id })) {
              sc.memory.jobPosted = false;
              return `laid off a hand at ${credits(dearest.hourly)}/h (contract ${dearest.contract})`;
            }
            return "the lay-off was refused";
          }),
        );
      }
      if (mine.treasury < dayOfWages && s.balance > 2 * dayOfWages) {
        const amount = Math.floor(s.balance / 4);
        candidates.push(
          doing("fund_firm", `put ${credits(amount)} of your own money, a quarter of it, into the firm's treasury so it can pay wages`, async (sc) =>
            (await sc.do(act.transfer, { to: { org: mine.id }, asset: { money: amount }, memo: "capital", on_behalf_of: null })) ? `moved ${credits(amount)} into the treasury` : "the transfer was refused",
          ),
        );
      }
      // The book for every good the firm holds or needs (SJ.5): what a price-setter reads.
      const unsold = daysUnsold(s, inv);
      const bookOn: Record<string, unknown> = {};
      // The inputs a unit consumes (a Builder's ten Materials a dwelling), one unit's worth, from the treasury:
      // taken at the best ask, or bid one tick under it and left resting (SJ.5).
      for (const [good, per] of Object.entries(consumes)) {
        const b = bookOf(books, good);
        const p = b.last;
        if (p === null || per <= 0 || (inv[good] ?? 0) >= per) continue;
        bookOn[good] = bookFacts(b);
        const take = b.bestAsk ?? Math.round(p * 1.05);
        const under = undercutPrice(b)!;
        if (mine.treasury < per * under) continue;
        const bid = (limit: number, label: string) => async (sc: Script) =>
          (await sc.do(act.placeOrder, { instrument: good, side: "bid", qty: per, limit_price: limit, on_behalf_of: mine.id })) ? `bid ${per} ${good} @${limit} for the firm (${label})` : `bid for ${good} refused`;
        const asking = b.bestAsk !== null ? `${b.askDepth} ${good} are asked at ${credits(b.bestAsk)}` : `nobody is asking ${good}; it last traded at ${credits(p)}`;
        if (mine.treasury >= per * take) {
          candidates.push(doing(`take_ask_${good}`, `buy ${per} ${good}, one ${produces}'s worth, at once at ${credits(take)} each: ${asking}; from the firm's treasury of ${credits(mine.treasury)}`, bid(take, "at the ask")));
        }
        candidates.push(doing(`bid_under_${good}`, `bid for ${per} ${good} at ${credits(under)} each, one tick under, and wait for a seller to come down to it`, bid(under, "under the ask")));
      }
      // The output: one tick under the best ask, so the shelf sells first, or 5% over the last print and wait (SJ.5).
      if (sellable.length) {
        const shelfWords = sellable.map(([g, q]) => `${q} ${g} unsold for ${unsold[g] ?? 0} day(s)`).join(", ");
        const under: [string, number, number][] = [];
        const over: [string, number, number][] = [];
        for (const [g, q] of sellable) {
          const b = bookOf(books, g);
          bookOn[g] = bookFacts(b, unsold[g] ?? 0);
          under.push([g, q, undercutPrice(b)!]);
          over.push([g, q, Math.round((b.last ?? b.bestAsk)! * 1.05)]);
        }
        const askAll = (asks: [string, number, number][], label: string) => async (sc: Script) => {
          const done: string[] = [];
          for (const [g, q, ask] of asks) {
            if (sc.actionsLeft <= 0) break;
            if (await sc.do(act.placeOrder, { instrument: g, side: "ask", qty: q, limit_price: ask, on_behalf_of: mine.id })) done.push(`asked ${q} ${g} @${ask} (${label})`);
          }
          return done.length ? done.join(", ") : "asks refused";
        };
        const bestAsks = sellable.map(([g]) => {
          const b = bookOf(books, g);
          return b.bestAsk !== null ? `${b.askDepth} ${g} asked at ${credits(b.bestAsk)}` : `no ${g} asked`;
        });
        candidates.push(
          doing("undercut", `ask ${under.map(([g, q, p]) => `${q} ${g} at ${credits(p)}`).join(", ")}, one tick under the best ask (${bestAsks.join("; ")}), so yours sells first: ${shelfWords}`, askAll(under, "undercut")),
        );
        candidates.push(doing("hold_price", `ask ${over.map(([g, q, p]) => `${q} ${g} at ${credits(p)}`).join(", ")}, 5% over the last price, and wait for the book to come to you`, askAll(over, "held")));
      }
      const perShare = Math.floor(mine.treasury / 4 / Math.max(1, mine.my_shares));
      if (mine.treasury > 3 * dayOfWages * 2 && perShare > 0) {
        candidates.push(
          doing("pay_dividend", `declare a dividend of ${credits(perShare)} a share, a quarter of a treasury of ${credits(mine.treasury)}`, async (sc) =>
            (await sc.do(act.dividend, { org: mine.id, per_share: perShare })) ? `declared ${credits(perShare)} a share` : "dividend refused",
          ),
        );
      }
      if (candidates.length === 0) return null;
      candidates.push(none("run_quietly", "change nothing at the firm this hour"));
      const output = yesterdayValue !== null ? `yesterday it made ${yesterdayUnits} ${produces}, worth ${credits(yesterdayValue)} at the last price` : `yesterday it made ${yesterdayUnits} ${produces}, which has no price yet`;
      return {
        key: "venture",
        tier: 2,
        ask:
          `what to do at the firm you manage. Its treasury holds ${credits(mine.treasury)} credits, ${coversDays === 0 ? "not enough to pay one worker for a day" : `enough to pay one worker for ${coversDays} day(s)`} at the going wage of ${credits(wage)} an hour; ` +
          `${workers} hired hand(s) at its ${wp?.kind ?? "workplace"}, costing about ${credits(wagesDay)} a day; ${output}; inventory ${stock.length ? stock.map(([g, q]) => `${q} ${g}`).join(", ") : "empty"}. You hold ${credits(s.balance)} credits yourself.`,
        facts: {
          managing: true,
          treasury: credits(mine.treasury),
          treasury_covers_worker_days: coversDays,
          workers,
          wages_per_day: credits(wagesDay),
          yesterday_output_units: yesterdayUnits,
          yesterday_output_value: yesterdayValue !== null ? credits(yesterdayValue) : null,
          inventory: Object.fromEntries(stock),
          going_wage: credits(wage),
          books: bookOn,
        },
        candidates,
      };
    }
    delete s.memory.firm;
    const need = orgs.founding.materials;
    const fee = orgs.founding.money;
    const have = s.pantry.materials ?? 0;
    const pm = lastPrice(books, "materials");
    const candidates: Candidate[] = [];
    // A chooser founds the workplace `which_workplace` picked (SJ.5), and not before it has picked one.
    const open = choose ? (s.memory.workplaceKind as WorkplaceKind | undefined) : kind;
    if (have >= need && s.balance >= fee && open) {
      candidates.push(
        doing(
          "found_now",
          `found a firm now, paying the ${credits(fee)} fee and the ${need} Materials, and open a ${open}${choose ? ", the workplace you chose" : ""}`,
          async (sc) => {
            const name = `${sc.handle}'s ${firmName}`;
            return (await sc.do(act.foundOrg, { kind: "firm", name, first_workplace: { kind: open, slot: null } })) ? `founded ${name} with a ${open}` : "founding refused";
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
}

/** The founder's and the borrower's firm: the workplace `which_workplace` chose (SJ.5). */
export const ventureSlot: SlotGen = ventureSlotOf("mine", "Works", true);
/** The builder's firm (SJ.3): a Builder, whose output is dwellings. */
export const builderVentureSlot: SlotGen = ventureSlotOf("builder", "Roofs");

/** A finished dwelling (SJ.3): sell it at cost plus a margin, or keep it and let it at the going rent — the accumulation decision the Materials tension exists for. */
export const dwellingSlot: SlotGen = async (s) => {
  const orgs = await s.orgs();
  if (!orgs) return null;
  const mine = orgs.orgs.find((o) => o.i_manage);
  if (!mine) return null;
  const free = mine.dwellings.filter((d) => d.offer == null && d.occupant == null);
  const d = free[0];
  if (!d) return null;
  const board = await s.board();
  const books = await s.books();
  const wage = median(jobOffers(board).map((o) => o.hourly)) || 800;
  const price = dwellingAskPrice(orgs, books, wage);
  const rent = goingRent(board);
  const daysToMatch = Math.ceil(price / Math.max(1, rent));
  return {
    key: "dwelling",
    tier: 2,
    ask:
      `whether to sell or let a dwelling your firm has finished. Sold, it would ask ${credits(price)}, its cost in Materials and hours plus a fifth, paid once; let, it would bring ${credits(rent)} a day, the going rent, ` +
      `so ${daysToMatch} days of rent equal the sale price. ${free.length} finished dwelling(s) stand empty and unoffered. The firm's treasury holds ${credits(mine.treasury)}.`,
    facts: { finished_unoffered: free.length, sale_price: credits(price), rent_per_day: credits(rent), days_of_rent_to_match_sale: daysToMatch, treasury: credits(mine.treasury) },
    candidates: [
      doing("sell_dwelling", `offer it for sale at ${credits(price)} and build the next`, async (sc) =>
        (await sc.do(act.offerSale, { asset: { dwelling: d.id }, price: { money: price }, to: null, on_behalf_of: mine.id })) ? `offered dwelling ${d.id} for sale at ${credits(price)}` : "the sale offer was refused",
      ),
      doing("lease_dwelling", `keep it and offer it to let at ${credits(rent)} a day`, async (sc) =>
        (await sc.do(act.offerLease, { asset: { dwelling: d.id }, rent_per_cycle: rent, term_cycles: null, on_behalf_of: mine.id })) ? `offered dwelling ${d.id} to let at ${credits(rent)} a day` : "the lease offer was refused",
      ),
      none("hold_dwelling", "leave it empty and decide another hour"),
    ],
  };
};

/** The lender's terms: a quarter of the balance, five days, whole credits. */
export const LEND_TERM = 5;

/** The lender's loan (SJ.3): a quarter of the balance on the board at 1% or 3% a day, once a day while none of mine is open, with the roll's defaults in the question. */
export const lendSlot: SlotGen = async (s) => {
  const board = await s.board();
  if (board.some((o) => o.kind === "credit" && (o.by as { citizen?: number }).citizen === s.me)) return null;
  const principal = Math.floor(s.balance / 4 / 100) * 100;
  if (principal < 5000) return null;
  if (!onceToday(s, "lend")) return null;
  const roll = await s.citizens();
  const defaulted = roll.filter((c) => (c.flags as { defaulted?: boolean }).defaulted).length;
  const interest = (bp: number) => Math.round(principal * (bp / 10000) * LEND_TERM);
  const lend = (bp: number, label: string) => async (sc: Script) =>
    (await sc.do(act.offerCredit, { principal, rate_per_cycle_bp: bp, term_cycles: LEND_TERM, collateral: null, to: null, on_behalf_of: null }))
      ? `offered ${credits(principal)} at ${bp / 100}% a day over ${LEND_TERM} days (${label})`
      : "the loan offer was refused";
  return {
    key: "lend",
    tier: 2,
    ask:
      `whether to offer a loan. You hold ${credits(s.balance)} credits; a quarter of it, ${credits(principal)}, could go out for ${LEND_TERM} days, repaid in daily installments. ` +
      `At 1% a day it would earn ${credits(interest(100))}; at 3%, ${credits(interest(300))}, if the borrower pays. Of ${roll.length} citizens on the roll, ${defaulted} have defaulted on a loan. ` +
      `A missed installment flags the borrower; this offer takes no collateral, so a default is your loss.`,
    facts: { principal: credits(principal), principal_share_of_balance: 0.25, term_days: LEND_TERM, interest_cheap: credits(interest(100)), interest_dear: credits(interest(300)), citizens_on_roll: roll.length, defaulted: defaulted },
    candidates: [
      doing("lend_cheap", `offer ${credits(principal)} at 1% a day, a rate anyone solvent can pay`, lend(100, "cheap"), true),
      doing("lend_dear", `offer ${credits(principal)} at 3% a day, dear enough to pay for a default`, lend(300, "dear"), true),
      none("hold_money", "lend nothing; keep the balance whole"),
    ],
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
  const day = dayWage(s, jobOffers(board));
  const rent = Math.max(100, Math.round(day * 0.2));
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
      `you own ${owned.length}, ${unleased.length} not yet offered to let. Balance ${credits(s.balance)} credits; a day's median wage is about ${credits(day)}.`,
    facts: { for_sale: sales.length, cheapest_affordable: cheapest ? credits(cheapest.price) : null, owned: owned.length, unleased: unleased.length, rent_would_be: credits(rent) },
    candidates,
  };
};

/** Comfort under 60 lifts Wares off the shelf (the preset's `wares_comfort_below`); each unit restores `comfort_per_wares`, six. */
export const COMFORT_LOW = 60;
export const COMFORT_PER_WARES = 6;

/** Comfort (SJ.5): every persona's first Wares demand. Once a day while Comfort is under 60, none are in the pantry and the balance covers them: take the ask, bid one tick under it, or go without. */
export const comfortSlot: SlotGen = async (s) => {
  const comfort = s.home.needs.comfort;
  if (comfort >= COMFORT_LOW || (s.pantry.wares ?? 0) > 0) return null;
  if (!onceToday(s, "comfort")) return null;
  const b = bookOf(await s.books(), "wares");
  const take = b.bestAsk ?? b.last;
  const under = undercutPrice(b);
  if (take === null || under === null) return null;
  // Enough to lift Comfort to 80, within five.
  const qty = Math.max(1, Math.min(5, Math.ceil((80 - comfort) / COMFORT_PER_WARES)));
  if (s.balance < qty * under) return null;
  const lift = qty * COMFORT_PER_WARES;
  const bid = (limit: number, label: string) => async (sc: Script) =>
    (await sc.do(act.placeOrder, { instrument: "wares", side: "bid", qty, limit_price: limit })) ? `bid ${qty} wares @${limit} (${label})` : "the Wares bid was refused";
  const candidates: Candidate[] = [];
  if (s.balance >= qty * take) candidates.push(doing("take_ask", `buy ${qty} Wares now at ${credits(take)} each, ${credits(qty * take)} in all, lifting Comfort by about ${lift}`, bid(take, "at the ask")));
  candidates.push(doing("bid_under", `bid ${credits(under)} each for ${qty} Wares, one tick under the ask, and wait for a seller to come down`, bid(under, "under the ask")));
  candidates.push(none("go_without", "go without Wares and keep the money"));
  return {
    key: "comfort",
    tier: 2,
    ask:
      `whether to buy Wares. Comfort is ${comfort.toFixed(0)} of 100 and falls 1 an hour; ${qty} Wares would lift it by about ${lift}. ` +
      `${b.bestAsk !== null ? `${b.askDepth} are asked at ${credits(b.bestAsk)}` : `none are asked; Wares last traded at ${credits(take)}`}${b.bestBid !== null ? `; the best bid is ${credits(b.bestBid)}` : ""}. Balance ${credits(s.balance)} credits.`,
    facts: { comfort: Math.round(comfort), wares_wanted: qty, comfort_lift: lift, cost_at_ask: credits(qty * take), wares: bookFacts(b) },
    candidates,
  };
};

/** The slots a persona is asked about, in tier order; a slug without its own list lives like a householder. */
const HOUSEHOLDER: SlotGen[] = [workSlot, housingSlot, jobSlot, comfortSlot, planSlot];
export const SLOTS: Record<string, SlotGen[]> = {
  founder: [workSlot, housingSlot, jobSlot, workplaceSlot, ventureSlot, comfortSlot, planSlot],
  borrower: [workSlot, housingSlot, jobSlot, creditSlot, workplaceSlot, ventureSlot, comfortSlot, planSlot],
  "wage-maximiser": HOUSEHOLDER,
  speculator: [workSlot, housingSlot, jobSlot, marketSlot, comfortSlot, planSlot],
  landlord: [workSlot, housingSlot, jobSlot, propertySlot, comfortSlot, planSlot],
  saver: HOUSEHOLDER,
  slacker: HOUSEHOLDER,
  lender: [workSlot, housingSlot, jobSlot, lendSlot, comfortSlot, planSlot],
  builder: [workSlot, housingSlot, jobSlot, builderVentureSlot, dwellingSlot, comfortSlot, planSlot],
};

export function slotsFor(slug: string): SlotGen[] {
  return SLOTS[slug] ?? HOUSEHOLDER;
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

export type { Offer };
