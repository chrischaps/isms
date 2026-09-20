# Phase 2 — The Commune: kickoff plan

Written 2026-09-19, before any card. The cards are TDD §18.5 S2.1–S2.11 (cut by ADR-0012); the design is GDD §6.2, §8 and §12; the provisional answers are Q116–Q122. No human playtest sits on Phase 2's gate.

## Context

Phase 1 closed on 2026-09-18 on its automated gates (ADR-0012). The Commune already runs headless: S0.15 built the Common Store, draw by need, the three rationing rules, the Ledger of Contribution, work norms, the Materials split as a policy field, and a stable householder Commune. What the sim set through `SetPolicy` from `Actor::System` (Q56) is what Phase 2 makes the assembly's: proposals, ballots, quorum, offices, and the coordinator's three powers, then a client and a voice for a society with no money.

What exists to reuse, by card:

- **Proposals.** One live path: `ProposalKind::Admission` (`crates/isms-core/src/world.rs`, the `Proposal` struct with `votes`), `bank.rs`'s `propose_admission` / `vote_admission` / `cycle_end_8j_close_proposals`, `apply.rs`'s handling, tick phase 8j. `ProposalClosed { proposal, passed }` exists. `ProposalId` exists. S2.1 generalises this path rather than adding a second one.
- **Offices.** `Offices { holders }` and `OfficeHolder { citizen, term_ends_cycle }` in `world.rs`; `OfficeKind`, `RecallRule`, `OfficeSpec { seats, term_cycles, consecutive, recall }` in `constitution.rs`; `Capabilities.offices`; `presets/commune.toml` already declares three coordinators, five-cycle terms, no consecutive, majority recall. Offices reset on `EpochStarted`.
- **Policy.** `Policy` carries `monitoring`, `work_norm_hours`, `rationing`, `materials_split`; `Policy::validate_against` rejects fields the constitution's axes do not use; `PolicyChanged` exists; `SetPolicy` is `System`-only (`command.rs`). All three rationing rules are implemented (`store.rs`, T8).
- **Vote default.** `VoteDefault::{Abstain, Follow(CitizenId), None}` on `StandingPlan` (`plan.rs`), editable on the Plan screen, executed by nothing.
- **Tunables.** `population.quorum_fraction = 0.20`, `population.office_vacancy_absent_cycles = 3`, `governance.coordinator_term_cycles = 5` in `presets/_base.toml`. `active_humans` is computed in `metrics.rs`.
- **Store and Ledger.** `CommonStore` and `Citizen.contribution: ContributionRecord` (`norms.rs`, phase 8i) exist in the engine with no server view and no screen.
- **Associations.** `credit.rs` and `orgs.rs::appoint_manager` carry the S0.11/S0.17 stubs ("member votes arrive in Phase 2"; Q90). There is no `Disburse` command: a manager disburses with a plain `Transfer` on the org's behalf.
- **The sim.** `planner::decide_system` returns nothing unless pricing is administered, so the Commune's policy never changes in the sim today. `stability_commune` in `crates/isms-sim/tests/sim.rs`.
- **Server.** Channels in `comms.rs` (`square`, `org:<id>`, `dm:`); TDD §12 already binds assembly threads to a `proposal_id`. Chronicle copy loads per preset from `presets/copy/<preset>/chronicle.toml`; only `freeport/` exists, so a Commune society cannot load copy today. `names.rs` names every refusal on the way out.
- **Web.** Screens mount on `useCapabilities()` and label through `useLexicon()` (`web/src/api/hooks.ts`); `web/src/screens/roles/` is reserved by TDD §4 and does not exist; `presets/lexicon/commune.json` is all TODO; `scripts/e2e/web.sh` seeds only Freeport.
- **Harness.** Personas are markdown with frontmatter, scripted strategies live in `agents/src/brain/scripted/strategies.ts`, `scripts/agents/run.sh` hard-codes `--preset freeport`.

## The cards, in order

| card | what | crates | runs beside |
|---|---|---|---|
| S2.1 | typed proposals, ballots, quorum, 8j close, `vote_default` | core (PR) | — |
| S2.2 | offices: elections, terms, rotation, recall, vacancy | core (PR) | — |
| S2.3 | coordinator's three powers; honors | core (PR) | S2.5 |
| S2.4 | association disbursement votes; the sim assembly; governance invariants | core + sim (PR) | S2.5, S2.6, S2.7 |
| S2.5 ∥ | proposal, ballot, office routes; `assembly:<pid>` channel | server | S2.3, S2.4 |
| S2.6 | Assembly screen; ballot builder (`roles/`); Commune Playwright seed | web | S2.4 |
| S2.7 ∥ | Store and Ledger views and screens | server + web | S2.2–S2.4 |
| S2.8 | Coordinator workspace | server + web | after S2.3 merges and S2.6 |
| S2.9 ∥ | Commune lexicon, Welcome Brief, Chronicle templates | presets + server | S2.6–S2.8 |
| S2.10 ∥ | Commune personas and tools; `make e2e-agents` on both presets | agents | S2.8, S2.9 |
| S2.11 | second-preset kit pass; `phase2-visual.md`; the exit run | web | — |

Full cards with Read, Build, Out of scope, Done gate and Hand-off: TDD §18.5.

## Sequencing and worktrees

- **Engine chain** S2.1 → S2.2 → S2.3 → S2.4 in `../Isms-p1`, one PR each on `s2.<n>-<slug>`, squash-merged on green Linux CI (ADR-0008). Every card keeps the proptest suite and `make sim-all` green; S2.1 and S2.2 regenerate goldens.
- **Server and web chain** on `main` through `make push`. S2.5 starts when S2.2 has merged (it needs the proposal and office types on the wire); S2.3's and S2.4's types land in a follow-up commit to S2.5 once they merge.
- **Two agents.** The second takes S2.5 then S2.7 in `../Isms-p2` while the first is on S2.3/S2.4; both touch `society_api.rs`, so S2.7 rebases on S2.5 before it starts and S2.8 waits for both. With one agent the order is the table's.
- **Seeds.** S2.6 adds a lab Commune to `scripts/e2e/web.sh`; S2.10 adds `PRESET` to `scripts/agents/run.sh`. A database seeded before S2.1 will not load its snapshot once `World` gains governance fields: reseed.
- **Budget.** About seven sequential slots for eleven cards with two agents; eleven with one. No model-driven harness run is planned (ADR-0011); S2.10's gate is the scripted cohort.

## Decisions to confirm

Provisional answers stand unless Chris says otherwise; each is one row in `docs/QUESTIONS.md`.

1. **Approval voting for elections** (Q118), chosen for its simplicity with three seats and a small electorate. The alternative is one-vote plurality.
2. **Only a coordinator may propose the rationing rule** (Q116), reading GDD §6.2's coordinator power as an exclusive one. The alternative reads it as "may also", and anyone proposes.
3. **The sim assembly is seeded scripted humans** (Q117's denominator, S2.4), so quorum and offices are real in `sim-check`. The alternative is a `System` shortcut that leaves quorum untested until the harness.
4. **Open and close a workplace are instant commands, not votes** (Q120), as GDD §6.2 lists them among the coordinator's powers.

## The Phase 2 exit

TDD §18.5 "Phase 2 exit": five automated lines on one sha of `main`, no human playtest. What a person must still judge goes into `docs/playtest/phase2-visual.md` as `[H]` marks for Phase 5 (ADR-0012).
