# The Jev brain — side-cards SJ.1–SJ.3

Written 2026-09-24. A side-series in `agents/` (no engine change), like SB.1–SB.5: each card commits to main and pushes on green. The decision is ADR-0014 (with its SJ.2 amendment); the brain is `agents/src/brain/jev/`; the runs are in `docs/playtest/runs/sj*.md`. One paid run per explicit approval stands (ADR-0011).

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
