# A market that moves — the sequence after SJ.3

Written 2026-09-24, from SJ.3's week (`docs/playtest/runs/sj3-7day.md`, SESSIONS SJ.3b) and the cards it produced (`docs/plans/jev.md`: SJ.5, SJ.6, E-1 to E-6). This is the order to do them in and why; the cards themselves stay where they are.

## The finding this answers

Sixteen players among twenty-four householders traded every good every day and moved no price: Food 1.31, grain 0.61, ore 0.92, Materials 2.00, Wares 4.12, machines 10.52, every day of seven. Every seller in every trade was a legacy firm; the players posted the plan's Food bids at the ask and ten other orders. The founders' asks at 5 % over the last price never filled after day 2 against a legacy ask that never ran dry. The price-setter is the householder runner's cost-plus formula (TDD 9.3), which does not respond to stock, and the players are its takers.

Two kinds of fix follow. **Levers** change what happens in the economy; **measurement** changes what we can see or how it scores. Levers go first, and the smallest lever whose absence explains the most goes before the rest.

## The sequence

| step | card | kind | where | cost | gate |
|---|---|---|---|---|---|
| 1 ✅ | **E-1** the runner's markup responds to stock — **done 2026-09-24** (`s3.1-runner-markup`, Q162; the shelf's step, `ShelfClosed` at 8m) | lever, engine | PR `s3.1-runner-markup` | a day; $0 | `make check`; `sim-check` all five presets stable; the founding curves (docs/tuning) re-read; green Linux CI |
| 2 | **SJ.5** prices from the players | lever, agents | main | a day; ~$0.15 on approval | the card's tests; `make e2e-agents`; a seven-day sixteen-player run read on its own price graph |
| 3 | **SJ.6** the town with eight householders | run | main (docs) | ~$0.15 on approval | the run clean; both weeks' price graphs side by side |
| 4 | **E-2** net worth counts dwellings and loans out (with Q157) | measurement, engine | PR | half a day | `scoreboard` tests; the archive's standings |
| 5 | **E-4** `seed --expect-humans` (Q161) · **E-3** an owner's position at founding (Q160) | cleanup, engine | one PR or two | half a day | `make e2e-agents`; the harness's token-wage trick and the day-1 emigration retired |
| 6 | **E-5** `/prices` across epochs · **E-6** the questions in the journal | observability | PR / agents | small | `pnpm --dir agents check`; a rollover read |
| — | **SJ.4** the Commune's slots | independent | main | ~$0.15 on approval | its own card |

## Why this order

- **E-1 before SJ.5.** SJ.5 lets a player undercut and post prices; against a runner whose asks are infinitely deep at a fixed formula, an undercut sells the player's own stock and the price snaps back the next hour — blips, not a market, and a second $0.15 spent measuring the runner's stubbornness. With E-1 the runner *answers*: a player's undercut pushes the mill's ask down, a sold-out shelf lifts it, and SJ.5's slots have something to push against. E-1 is also the smallest engine card (one rule in `householder.rs`, one tunable in `presets/_base.toml`) and the one whose absence explains the whole flat table.
- **SJ.5's graph first within SJ.5.** Build step 4 (per-good price rows in `days.jsonl`, "Prices, day by day" in the report) before the pricing slots, so E-1's effect is visible in the free scripted runs before a paid one.
- **SJ.6 is cheap and sits on SJ.5's graph.** Fewer householders makes the players the labour force; wages are the price that should have moved this week and did not. Run it only once there is a graph to read it on.
- **E-2 to E-6 are not levers.** E-2 changes how the landlord and the lender *score* (628.47 and 583.39 this week after thirteen dwellings and one loan), not what happens; E-3 and E-4 retire harness tricks that work today; E-5 and E-6 are observability. None would change what the day table shows, so they follow when the sim is open rather than before the levers.
- **SJ.4 is independent.** The Commune has no money and no books; nothing here touches it. It goes whenever a change of subject is wanted.

## What "done" looks like

The liveliness question is answered when a seven-day run's price table shows, for at least one good, a day-VWAP that differs between two days by more than the runner's cost-plus step, with a player on the moving side of the trade — and the answer is read from the graph, not the database. If after steps 1–3 the basket is still a line, the next lever is the recipes' start prices (the sim's, not the harness's), and that is a tuning card for Phase 3, not another SJ.

## Rules that stand

- Engine changes are PRs on green Linux CI (CLAUDE.md); the runner is shared with the sim, so E-1 must leave every preset stable under `sim-check` before it reaches main.
- One paid run per explicit approval (ADR-0011); every run's report to `docs/playtest/runs/`; read a lab week from day 2 until E-4 lands (Q161).
- Structural over cheesing: no seed or preset lever to make a price move that a player or the runner could not have moved themselves.
