# The Jev brain — side-cards SJ.1–SJ.6, and the engine cards they hand on

Written 2026-09-24; the order to do the cards after SJ.3 is in `docs/plans/market-liveliness.md`. A side-series in `agents/` (no engine change), like SB.1–SB.5: each card commits to main and pushes on green. The decision is ADR-0014 (with its SJ.2 amendment); the brain is `agents/src/brain/jev/`; the runs are in `docs/playtest/runs/sj*.md`. One paid run per explicit approval stands (ADR-0011).

## Context

TypeSafe's Jev is a decision model that writes no text: one `POST` with a JSON `state` and a map of typed `choice` questions, answered in one pass with a probability each, ~300 ms and ~$0.00005 a turn through OpenRouter. The brain lays the hour out as *slots* — each option a tool call with its numbers worked out in code, one sentence Jev reads, a do-nothing option at the end — and Jev picks. It supplies the persona-flavoured judgment the scripts hard-code; it cannot supply the comprehension data (`did_not_understand`) that stays the language model's.

What the runs so far have shown (`sj1-*`, `sj2-*`, six runs, $0.15):

- **The wiring, price and speed are settled.** 1,162 turns in a seven-day epoch for five cents, no skipped hours at a 10 s tick, one gateway 520 (now retried).
- **Jev's picks are sensible where a choice is real.** The founder saved with confidence climbing until it could buy Materials, was held back once by the irreversible floor and founded at 0.51; every player rented a roof within hours of being offered one; the wage-maximiser switched when a firm posted at the generous tier.
- **The lab is a healthy, flat town.** Food 1.31 and the price index 1.14 every day of seven; credit 0.00 every day; twenty firms every day; Gini 0.02–0.05; needs met 98%. Prices are the legacy runner's cost-plus asks; nobody lends; nobody sells a dwelling. Seven players are price-takers among 32 scripted householders and 18 legacy firms.
- **The founded mines lose money**, and the reason is in the slots: the founder hired two workers at the median wage, sold ~4 ore a day, and never worked its own mine, because `work` only knows contract jobs. Both founders ended 38th and 40th by net worth; the five players who only worked and saved ended 1st–5th.
- **Day 5 of `sj2-7day` broke the pattern** (Materials −63%, unemployed 2 → 9, investment share 5% → 13%) and is undiagnosed: the aggregates alone do not say why.

## The cards

| card | what | done gate | |
|---|---|---|---|
| SJ.1 | `JevBrain` behind the `Brain` seam; `work`/`job`/`venture`/`market`/`property` slots; the client; floors; the report's Jev table; the founder-1 replay | 16 tests; a two-day and a seven-day run clean | done 2026-09-24, $0.04 |
| SJ.2 | slots only for a reason; `housing`, `plan`, `credit`, `fund_firm`, two wage tiers; `standing` from the scoreboard; `ambition`; all seven personas; the lab seeds a dwelling per player (Q159); the economy day by day and the standings in the report | 8 tests; two-day and seven-day runs clean | done 2026-09-24, $0.11 |
| SJ.3 | founders that can decide; a lender; a seller; a town of players; the day's headlines | below | done 2026-09-24, $0.14 |
| SJ.4 | the Commune's slots: ballots, offices, the Plan, the Store | later | |
| SJ.5 | prices from the players: `undercut`/`hold_price`, resting bids, a workplace chosen by margin, `job` re-asked while unemployed, the price graph in the report | below | done 2026-09-25, $0.22 over two weeks (SESSIONS SJ.5, SJ.5b, SJ.5c) |
| SJ.6 | the town with eight householders (`POPULATION_FLOOR=24`), read on the price graph | below | done 2026-09-25, $0.15; the town starved (SESSIONS SJ.6) |

## SJ.3 — Founders that decide, a lender, a seller, and a town of players

- **Goal.** Put the missing sides of the market into the lab — so that a founder can work and price its own firm, credit and property have counterparties, and the players are the town rather than visitors to it — and read the seven-day economy table for a price that moves.
- **Read.** This plan; `docs/decisions/0014-a-typed-decision-brain.md`; `docs/SESSIONS.md` SJ.2 and SJ.2b; `docs/playtest/runs/sj2-7day.md`; `agents/src/brain/jev/candidates.ts`; `agents/src/brain/scripted/strategies.ts` (`founder`, `landlord`, `borrower`); `docs/tdd.md` §9.3 (the householder runner: what it prices, hires, leases, never lends, never sells); the engine's `SetLabor` validation (`crates/isms-core/src/handle*.rs`, whoever is allowed to allocate hours at a workplace) and `Terminate` for an employer; `presets/_base.toml` recipes (`mine`, `builder`, `base_rate`, founding costs).
- **Build.**
  1. **The manager's own workplace in `work`.** First settle whether a citizen may `set_labor` at a workplace of an org it manages without a contract (read the engine; if not, Q160 and the founder accepts its own firm's job offer, or the card records that it cannot). Then `work` offers, for a manager, `work_own` (hours at its own workplace) beside the contract job, and the firm question says what a day's wages cost against what yesterday's output sold for (inventory delta by day in `memory`, times the last price). Add `lay_off` (terminate a worker's contract on the org's behalf) when wages exceed output value. Fixture: an orgs view with `i_manage` and a workplace with workers (`test/fixtures/api/GET_s_1_orgs.json` amended or a second fixture).
  2. **A lender persona.** `personas/lender.md` (ambition: income from interest, never a default); a scripted strategy (post one credit offer of a fixed principal at a fixed rate when the balance covers twice the principal and none is open; collect) and a `lend` slot for Jev: `lend_cheap` (1% a day), `lend_dear` (3% a day), `hold`, with the principal as a share of the balance and the borrower's default flag count from the citizens roll in the question. The borrower's `credit` slot now has something to read.
  3. **A seller of dwellings.** `personas/builder.md`: founds a Builder (the founder's road with `first_workplace: builder`), works it or hires, and when a dwelling is built chooses `sell_dwelling` (at book cost plus a margin) or `lease_dwelling` (at the legacy rent) — the accumulation decision the GDD says the Materials tension is for. The landlord's `property` slot now has something to buy. A dwelling may not appear within seven days at `base_rate` 0.5; the card records what it took as [H].
  4. **A town of players.** `run.sh` takes `POPULATION_FLOOR` (`--param params.population.floor`) and the persona list cycles to `PLAYERS`; `[jev].personas` gains `lender` and `builder`; `personas_for.freeport-town` or a `--cast` flag names a sixteen-player cast (two founders, two lenders, a builder, a landlord, two speculators, two wage-maximisers, two savers, two slackers, a borrower, the rule-prober). `initial_dwellings` follows `40 + PLAYERS` as now.
  5. **The day's headlines in the day table.** The cohort's day-end snapshot also reads `/chronicle` for the day just closed and keeps its headline texts in `days.jsonl`; the report prints them under the table, so a day like `sj2-7day`'s fifth can be read, not guessed.
  6. **Per-slot floors.** `[jev].floors = { market = 0.5 }` overrides `min_confidence` for a named slot; `market` is the one slot that flips.
- **Out of scope.** The Commune's slots (SJ.4); a daily reflection whose notes enter the state; any change to the householder runner or the engine (a Builder that sells is a persona, not the runner; a runner that lends is an engine question for a card); tuning the recipes because founded mines lose money at start prices — that is a finding to hand to the sim, not a thing to fix here.
- **Done gate.** `pnpm --dir agents check` green with tests named: `work_own` offered to a manager and its `set_labor` input; `lay_off` only when wages exceed output; the lender's slot and its `post_credit_offer` input; the builder's `sell_dwelling`/`lease_dwelling` after a build; the cast of sixteen loads; the headlines land in `days.jsonl` and the table. `make e2e-agents` green (scripted path, both presets). Then, on approval: a two-day sixteen-player run (~$0.05) clean — no error, no 5xx, `credit` above zero on the day table, a founder with hours at its own workplace; then a seven-day run (~$0.15) read for: a Food or Materials price that differs between days, credit outstanding above zero, at least one dwelling sold or the [H] on why not, the founders' book value not falling day on day, a Gini that opens, and day-5-style breaks explained by their headlines. Stop at $1 either way.
- **Hand-off.** SJ.4 needs the Commune slots' wire shapes (`agents/test/fixtures/commune/`); the `lend` and `sell_dwelling` decisions are the first two-way choices with real stakes on both sides, so their confidences are the ones to read for calibration; whatever the founders' mines show at seven days is the number to hand the sim.

## What SJ.3's week said (2026-09-24, from the event log)

Every good traded every day — Food 400–1,560 trades a day, grain and ore ~75, Materials 40–170, machines ~20, Wares thin — and every good traded at exactly one price all week (Food 1.31, grain 0.61, ore 0.92, Materials 2.00, Wares 4.12, machines 10.52); the only movement was the runner's cost-plus re-run on days 1–2. Every seller in every trade was a legacy firm; the players posted 2,312 Food bids at the ask (the plan's) and ten other orders. The founders asked ore at 0.97 against a legacy ask of 0.92 that never ran dry, so nothing they made after day 2 sold. The price-setter is a formula with no response to stock, and the players are its takers. The cards below are the fixes, in order of leverage; the engine ones are PRs (CLAUDE.md) and belong to whoever holds the sim.

## SJ.5 — Prices from the players

- **Goal.** Make the players price-setters where the engine already lets them, so the week's price graph has something on it before the runner is touched.
- **Read.** This plan (SJ.3's section and the paragraph above); `docs/SESSIONS.md` SJ.3 and SJ.3b; `agents/src/brain/jev/candidates.ts` (`ventureSlotOf`, `jobSlot`, `marketSlot`); `crates/isms-core/src/market.rs` (continuous matching: a new order fills against the resting best at the resting price, ADR-0001).
- **Build.**
  1. **Pricing in the firm slot.** `sell_output` becomes two candidates with the book in the sentence: `undercut` (one tick under the best ask; "N ore unsold for D days" from the inventory kept in `memory` by day) and `hold_price` (5% over the last price, as now); `buy_<input>` likewise `bid_under` (one tick under the best ask, resting) and `take_ask`. The Jev state carries best bid, best ask and days unsold per good the firm holds.
  2. **A workplace chosen by margin.** Before `found_now`, a `which_workplace` slot once a day when the Materials and the fee are in hand: mine, mill, workshop, foundry, each with its margin precomputed from `/orgs` recipes and the books (product's last price × base rate × 8 h − inputs at the last price − a day's wage), the best first; the founder and the borrower found what they chose; `found_now`'s sentence names it.
  3. **`job` while unemployed.** The slot is re-asked each day it is declined with "you have had no job for D days and D × a day's wage has gone unearned" in the sentence, so `wait` is a change with a cost, not a resting state; `wait` stays the do-nothing option.
  4. **The price graph in the report.** The cohort's day-end snapshot adds a per-good row from `/books` (last price, best bid, best ask, depth) to `days.jsonl`; the report draws "Prices, day by day" (one column per good) under the economy table, so the flat line is visible without the database.
  5. **Comfort.** A `comfort` slot for every persona when Comfort is under 60 and the balance covers it: buy Wares at the ask, or bid under it, or go without; the players' first Wares demand.
- **Out of scope.** Any change to the runner or the engine (E-1 to E-5 below); the Commune's slots (SJ.4).
- **Done gate.** `pnpm --dir agents check` green with tests named: `undercut` one tick under the best ask and its `place_order` input; `which_workplace` ranks by margin and `found_now` follows it; `job` re-asked while unemployed with the days in the sentence; the price rows in `days.jsonl` and the table; the `comfort` slot. `make e2e-agents` green. Then, on approval, a seven-day sixteen-player run (~$0.15) read for: a founder's output that sells after day 2, at least one good whose day-VWAP differs between two days by more than the runner's cost-plus step, a lender with a job.
- **Hand-off.** SJ.6 runs the same cast with fewer householders; the read is the price graph this card draws.

## SJ.6 — The town with eight householders

- **Goal.** Make the players the labour force and see whether wages move.
- **Build.** A run, not code: `POPULATION_FLOOR=24 PLAYERS=16 CAST=freeport-town BRAIN=jev` for seven days (~$0.15, on approval); the day table and the price graph read against SJ.3's week. If legacy firms cannot hire, the day wage is the number to watch; if they idle, that is the finding for E-1.
- **Done gate.** The run clean; the read in SESSIONS; the two weeks' price graphs side by side in the report of the second.

## Engine cards handed to the sim (PRs, `s3.<n>-<slug>` branches)

| card | what | why | source |
|---|---|---|---|
| E-1 | **The runner's markup responds to stock** (TDD 9.3): cut the ask when a legacy firm's inventory grows day on day, raise it when the shelf sold out; the response as a tunable in `presets/_base.toml`, `sim-check` on all five presets | nothing can move a price down but a player undercutting, and nothing moves one up at all; the single largest lever on the flat basket | SJ.3b · **done** 2026-09-24 (`s3.1-runner-markup`, Q162) |
| E-2 | **Net worth counts dwellings and loans out** (`scoreboard`, GDD 6.2), with Q157's contribution totals | the landlord and the lender read as losses by construction (628.47 and 583.39 after thirteen dwellings and one loan) | SJ.3b, Q157 · **done** 2026-09-25 (`s3.4-scoreboard-holdings`, Q166: dwellings at the last sale else build cost, loans at the unpaid principal on both sides, in net worth and book value; `Standing.contribution` by norm) |
| E-3 | **An owner's position at founding** (Q160): `FoundOrg` emits `Assigned { contract: None }` for the founder at the first workplace, as a cooperative member's | the harness hires the founder at a token wage to get its hours counted; the game should not need the trick | SJ.3, Q160 · **done** 2026-09-25 (same PR; `work_own` sets hours at the position, the one-cent self-hire is gone) |
| E-4 | **A seed that expects its players** (Q161): `seed --expect-humans N` fills the floor to `floor − N` | every lab run emigrates one householder per player at day 1's end and reads day 2 as a slump | SJ.3, Q161 · **done** 2026-09-25 (`s3.2-seed-expect-humans`, `population.expected_humans`) |
| E-5 | **`/prices` across epochs**: the window may cross a rollover, or the archive keeps the epoch's per-good series | after a rollover the week's prices are only in the event log | SJ.3b · **done** 2026-09-25 (main: the window runs back across a rollover, every point carries its `epoch`; `?epoch=N` keeps it inside one epoch, so an archived epoch's last days stay readable; the `prices` tool takes `epoch`) |
| E-7 | **The runner's wage answers its staffing**: raise a workplace's offer a step when it closes short of hands, cut it when the shelf grew; post offers when idle hands, not full shelves, are the reason (Q2); a tunable step in `presets/_base.toml` | `sj6-7day`: fifteen payrolls missed on day 2 and every offer at 8.00 all week; the day wage fell 54 → 33 only because fewer hours were paid; E-1's labour-side twin | SJ.6 · **done** 2026-09-25 (`s3.3-runner-wage`, Q164, Q165: `Workplace.wage_step`, `WageStepped` at 8m, offers withdrawn when the firm cannot honour them) |
| E-6 | **The questions in the journal**: a Jev turn's request (state and questions) kept beside its decisions, or a `--journal-questions` flag | whether `lay_off` was ever offered is unreadable from `sj3-7day` | SJ.3b · **done** 2026-09-25 (main: every decision carries `offered`, the slot's options in the order asked, and the report's slot table lists what was offered beside what was chosen; `--journal-questions` / `jev.journal_questions` keeps the whole request — state and questions verbatim — as `request` on the turn) |

## Rules of the series

- Jev never sees an id, a quantity it would have to compute, or a price it would have to compare; the comparison is in the option's sentence, and every slot ends in a do-nothing option.
- The persona and its ambition live in the question's instructions, never in the state.
- A slot is offered only when something gives a reason to choose; a declined daily question is not re-asked until tomorrow.
- The state builder is a pure function of the view, so the replay test holds; re-record `founder-1` when the request order changes.
- One lab server at a time on this machine, and never edit `run.sh` while a run is executing it.
- Every run's report goes to `docs/playtest/runs/`; the SESSIONS entry carries the numbers.

## Risks noted

- If the engine refuses a manager's own hours without a contract, the founder must hire itself, which the API may refuse as a self-deal (Q160 either way).
- A Builder at `base_rate` 0.5 with 10 Materials a dwelling may not finish one in seven days; the landlord may still have nothing to buy.
- Sixteen players against twenty-four householders changes the town's prices only if the players *post*; a cast that only accepts offers is still price-taking. Watch the day table, not the standings.
- A lender's counterparty risk is the borrower's default; the card measures it, it does not prevent it.
