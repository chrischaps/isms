# Game Design Document: *Isms* (working title)

*A web-based multiplayer economic simulation exploring how people behave under different economic systems.*

**Status:** GDD v0.1 — first full skeleton · **Author:** Chris (drafted with Claude) · **Date:** Sept 2026
**Upstream:** [Concept Doc](economic-systems-game-concept.md) (Sept 2026) · **Downstream:** Technical Design Doc (future)

**How to read this doc.** Every section carries a status tag. **[Fixed]** means it's inherited from the concept doc and not up for debate here. **[Proposed]** means this doc takes a position and gives the reasoning; treat it as a decision pending your sign-off. **[Open]** means it's still a real question, usually deferred to playtesting or the TDD. Numbers in this doc are starting points for a tuning simulation, not final values — the *shape* of each rule is the decision, the constants are not.

---

## 0. Table of Contents

1. Overview & Pillars
2. Decisions on the Concept Doc's Open Questions
3. Glossary
4. The Material Base (commodities, needs, labor, capital, land)
5. The System Axes (rigorous definitions)
6. Shipping Presets (full rule specs)
7. Organizations, Contracts & Transfers
8. Governance & Special Roles
9. Player Experience (core loop, sessions, standing plan, onboarding)
10. Success, Progression & Scoreboards
11. Multiplayer Structure (societies, time, population, AI citizens, agents, epochs)
12. Communication
13. Identity, Alts, Trust & Safety
14. Measurement, Observatory & Telemetry
15. Interface Design — the UI embodies the system
16. Expected Emergence (and the primitives that permit it)
17. Balance & Tuning Approach
18. Scope, MVP & Prototype Plan
19. Risks
20. Remaining Open Questions
Appendix A. Parameter Table
Appendix B. Neutrality Review Checklist

---

## 1. Overview & Pillars **[Fixed]**

Players join a persistent online society and live in its economy: work, get paid, consume, save, and — where the rules allow — trade, invest, organize, and govern. Each society runs one economic system, defined as a configuration across a set of axes. Named systems (Pure Capitalism, Pure Communism, and hybrids) are presets. The game is rendered entirely as UI — dashboards, forms, charts, feeds. The economy *is* the game.

**North star:** the fun is *feeling how the system shapes your experience as a participant.* A player who has lived a week in each society should come away with a first-person sense of both.

**Pillars (from the concept doc, restated as design tests):**

| Pillar | Test applied to every feature in this doc |
|---|---|
| Authenticity is the fun | Would a committed advocate of this system call the implementation fair? (Appendix B) |
| Felt experience beats abstract accuracy | Does this rule make the system's character *legible in play*, or just more correct? |
| Fidelity to the idea, not to history | Are we modeling the system as theorized, not a caricature of a regime? |
| Fun through agency, not through tilt | Does the player have a meaningful choice *within* the rules here? |
| Every system defines its own success | Have we avoided smuggling in a universal victory metric? |
| UI-only, forever | Does this need a world? Then it's out. |
| Legible state | Can every number on screen be explained by a rule plus player actions? |

**Platform:** web browser, desktop-first, responsive enough for a phone check-in. **Audience:** curious adults who enjoy systems games (Eve players, Frostpunk/Victoria fans, Taskman users), economics-adjacent students and educators, and people who will show up for a social experiment.

---

## 2. Decisions on the Concept Doc's Open Questions **[Proposed]**

This section is the heart of the GDD pass: each of the ten §10 questions gets a recommended answer and its reasoning. Later sections elaborate mechanics; this section is the place to disagree.

### Q1. What is the labor interaction? What stops botting from trivializing it?

**Decision: labor is an allocation-and-effort decision, not an active task. Botting is not prevented; it's made pointless.**

The player's labor decisions each cycle are (a) *where* to work (job/workplace, within what the system allows), (b) *how much* (hours, up to a daily budget), and (c) *how hard* (effort level: low / normal / high). Output is a function of hours × skill × effort × workplace productivity. There is no minigame.

Reasoning. An active task in a pure-UI game would reward dexterity, not economic judgment, and it's exactly what a bot trivializes. But the interesting labor variable in economics was never "can you click fast" — it's *effort under different incentive structures*. Shirking is the whole story of the principal-agent problem, the piece-rate debate, the quota ratchet, and "they pretend to pay us and we pretend to work." So the design puts the decision that matters — effort, with real tradeoffs — directly in the player's hands, and makes it observable in different ways under different systems. Under piece-rate wages, low effort costs you; under equal pay, it doesn't; under a plan with quotas, it costs your workplace's bonus. The system shapes the choice; the player feels it.

Effort has costs so it's a real choice: high effort raises output (×1.3) but also raises need consumption (you're hungrier) and accrues fatigue that lowers next-cycle capacity. Low effort (×0.6) saves both. Whether anyone *notices* your effort depends on the system's monitoring rules (§4.3, §5).

Botting: because labor is a standing decision that the game already executes for absent players (the Standing Plan, §9.3), a bot gains nothing over the built-in automation on the labor side. Where an agent *does* gain is market reactivity (repricing, sniping bids, arbitrage). That's a legitimate strategic advantage — it's what high-frequency traders have — and it's an experimental variable worth measuring rather than a flaw to patch. So: the UI is a client of the public API, agents are allowed as *proxies for a human's single citizen* (Q5), and their presence is recorded. Societies can be created with agents disallowed if a clean human-only reading is wanted.

### Q2. What operationalizes "pure communism" and "pure capitalism"? Which presets ship first?

**Decision: eight axes with enumerated settings (§5); five presets ship in v1 (§6): Pure Capitalism, Pure Communism, Central Planning (State Socialism), Social Democracy, Market Socialism.** Prototype order: Pure Capitalism → Pure Communism → Social Democracy → the other two.

The definitional stance, in one paragraph each:

*Pure Capitalism* = private ownership of all means of production (including land), free price formation on open order books, wages set by individual contract, free job choice, open capital markets (equity and credit between any parties), zero redistribution, and no economic governance at all — the only "state" is the engine enforcing property and contracts. Unclaimed land is acquired by homesteading (Lockean first-use), then only by purchase. Nothing to vote on; nobody to appeal to.

*Pure Communism* = the system as theorized in its higher phase: collective ownership of all means of production, no money and no prices, labor freely contributed according to ability under publicly known work norms, distribution from a common store according to need, no capital markets (investment is a collective decision about how much output to divert to machines), and governance by direct assembly with elected, recallable, rotating coordinators who can plan but cannot compel. This is deliberately *not* Soviet-style planning — that's a separate preset, Central Planning, because most people who say "communism" mean that, and the contrast between the two is itself educational.

Why these five: they span the axes without redundancy. Pure Capitalism and Pure Communism are the poles. Central Planning isolates *money + planning + state ownership* from *moneyless distribution by need*. Social Democracy isolates *redistribution + democratic policy* on top of markets. Market Socialism isolates *ownership* (worker coops) while holding *price formation* at market — the cleanest single-axis flip from capitalism. State Capitalism and Anarcho-syndicalism are natural v2 presets (each is a one-axis move from a shipped one).

The rigor requirement is met in §5 by defining each axis setting as a concrete rule in the engine, and in §6 by writing each preset as a full rule spec that a partisan could audit.

### Q3. What are the consequences of unmet needs?

**Decision: deprivation costs capacity and visibility, never progress. Nothing is permanent, nobody dies, no account is lost.**

Three needs (Food, Shelter, Comfort — §4.2), each a 0–100 satisfaction meter. Consequences tier up with duration:

1. *Immediate, reversible:* low Food and Shelter meters multiply labor output down (to a floor of ×0.4). Eat, and it recovers next tick.
2. *Hardship (Food < 20 for a full cycle):* the citizen's labor budget shrinks for the next cycle (fatigue debt) and they are publicly flagged "in hardship." This is the visible stake — for them and for everyone else.
3. *Destitution (hardship for 3+ cycles):* Comfort collapses, output floor drops to ×0.25, and — this is the one that bites — the citizen's *options* narrow: they can't run for office, can't found a firm, can't sign long contracts. Deprivation removes agency, which is the truest thing about it.

Why this shape. Stakes must be real enough that a Food shortage in a planned society, or unemployment in a market one, is *felt* — otherwise no system means anything. But permanent loss (death, wipe, debt spiral you can't exit) drives churn and, worse, drives away exactly the players in the bottom decile whose experience is the point. Capacity loss preserves the incentive gradient without an exit ramp. And the societal stakes carry the rest: need-fulfillment rate is a headline public metric, and other players' hardship flags are visible, so a society's failure to feed its members is a *story*, not just a stat. Whether anyone helps is the experiment.

### Q4. How are special roles filled, and how is abuse handled?

**Decision: filling rules are part of each preset's constitution (§8); abuse *within* the rules is data, abuse *of* the platform is moderated.**

Per preset: Pure Capitalism has no offices — founders self-select by putting up capital. Pure Communism elects coordinators by assembly, with short terms, rotation limits, and instant recall. Central Planning elects a three-seat Planning Committee for a longer term with full plan authority between elections (democratic centralism). Social Democracy elects a legislature that sets policy parameters. Market Socialism has coop-level elected managers and a publicly governed investment bank.

The line on abuse: a planner who allocates themselves a bigger ration, an employer who pays starvation wages, an assembly that votes to expel a shirker — all in-system, all recorded, all *exactly what we want to observe*. The system's own recourse (recall, quit, organize) is the answer, and whether it works is a finding. What the engine forbids is action outside the system's axes: no role can change constitutional settings, ban a player, read private messages, or alter telemetry. Vacancies degrade gracefully: an absent office-holder is auto-vacated after N cycles, the last plan or policy stands until replaced, and AI citizens never hold office (so a dead society doesn't get an AI dictator).

### Q5. Population: minimum size, AI-citizen policy, player-run agents?

**Decision: floor of 40 citizens (humans + AI fill), comfortable at 100–300, cap 500. AI citizens are transparent rule-followers with no vote and no office. Player agents are permitted only as a proxy for that player's one citizen.**

The floor comes from market thickness: seven workplace types (§4.4), and a market economy needs at least three competing producers per traded good to avoid trivial monopoly, so ~21 workplaces at 2–4 workers each → roughly 40–80 workers. AI citizens ("householders") fill the gap and taper out as humans join (AI count = max(0, floor − active humans)). They follow a published script: take the best available job or their assignment, work normal effort, buy needs, save 10%, hold no office, cast no vote. In market systems, a set of *legacy firms* run by AI householders on transparent cost-plus pricing seeds the economy and are for sale at book value — so early humans have somewhere to work and something to buy.

On player agents: forbidding them is unenforceable and would make the API — a core part of your design philosophy — a lie. Fielding *extra* citizens via agents, though, breaks one-person-one-citizen and contaminates data. The resolution is that an agent is a client of the same API the UI uses, authenticated as the human's citizen, and the fraction of a citizen's actions taken via API key is a recorded telemetry field. Societies carry an `agents_allowed` flag. The research question shifts honestly from "humans" to "humans, some tool-assisted," which is the world we live in. A separate "strategy league" track — agent-only societies — is a v2 idea, kept out of the canonical comparison.

### Q6. Tick length and session cadence?

**Decision: three nested clocks. Tick = 1 real hour (production, consumption, market clearing resolve). Cycle = 1 real day = 24 ticks (payday, quota day, ration day, votes close). Epoch = 6 real weeks (society lifecycle, Q7).**

Why hourly ticks: a deep evening session sees three or four ticks resolve, so markets visibly move while you watch and your decisions get feedback within the session. Why daily cycles: the economy's "day" *is* the player's day, so a 10-minute check-in has a natural agenda — see the payday, check the ration, adjust the plan, read the Chronicle — and nothing important resolves faster than daily except what the Standing Plan already handles. Labor is budgeted per cycle (8 hours) and executed per tick per the plan, so sleeping costs nothing. Real-time activity (trades, chat, proposals) is continuous between ticks. Alternatives considered: 4-hour ticks (markets feel dead in a session) and 15-minute ticks (check-in players fall behind and the standing plan does everything — presence stops mattering).

### Q7. Epochs vs. permanence?

**Decision: canonical societies run in fixed-length epochs (6 weeks) with a clean reset and a published Chronicle. Community societies may be permanent.**

Clean comparison needs equal starting conditions and bounded duration. Late joiners need a fair start — joining week five of a capitalist society where all the land is owned is a bad first experience, and a fresh epoch fixes it. Failed societies need an ending with dignity, and "collapse" (active humans below floor for 5 consecutive cycles) ends an epoch early. What carries across epochs: identity, reputation, honors, and a record ("served two terms as coordinator in Commune-3, epoch 4"). What doesn't: any material state. The epoch end itself is a designed moment — the last cycle is announced, the Chronicle is generated, the observatory snapshot is taken, and citizens can leave a closing statement that goes into the archive.

### Q8. Communication?

**Decision: in-game structured communication is mandatory for anything with mechanical effect; free-text chat exists in-game; Discord will happen anyway and is not fought. Communication affordances are configurable per society and are a declared experimental variable.**

Anything that *does* something — a job offer, a sale, a proposal, a vote, a contract, a recall — is an in-game structured object, so it's logged and it's legible. Free-text channels exist (Square, org channels, DMs, Assembly floor) so that the informal economy and politics have a home we can see. The Chronicle — an auto-generated news feed of economic events — is both a fun driver and the main substitute for a shared world. Canonical societies ship with the standard channel set; variant societies can restrict (no DMs, assembly-only) as a deliberate manipulation later.

### Q9. Identity & alts?

**Decision: one citizen per person per society, enforced by verified accounts on canonical societies; alts flagged and excluded from data before they're banned.**

Pseudonymous handles, persistent across societies, so reputation travels. Canonical (research) societies require a verified account (OAuth plus one more signal — phone or payment-free identity check, TDD question). Open societies don't. Alt detection is a telemetry problem (shared devices, mirrored action timing, one-way transfers); flagged citizens are excluded from research datasets first and moderated second. Players consent at signup to aggregate behavioral study; that consent text is a design deliverable, not boilerplate.

### Q10. Endogenous reform?

**Decision: split each society's rules into a Constitution (the axis settings) and Policy (parameters the system's own governance can move). Canonical societies have a fixed constitution and live policy. Community societies may amend the constitution through their governance, and are labeled as drifted.**

This keeps the comparison clean without freezing the systems into dioramas. A social democracy whose legislature can't touch the tax rate isn't a social democracy; a planned economy whose committee can't revise the plan isn't planned. That's *policy* and it's live everywhere. But a social democracy that votes to abolish private ownership has become something else — that's a *constitutional* change, and in a canonical society it's off the table so that "Social Democracy, epoch 7" means the same thing as "Social Democracy, epoch 2." Community societies get amendments as a feature (and their drift is itself interesting data, kept separate).

---

## 3. Glossary

| Term | Meaning |
|---|---|
| Society | One server, one system, one population, one closed economy. |
| Citizen | A player's (or AI householder's) presence in one society. One per person per society. |
| Constitution | The society's axis settings (§5). Fixed in canonical societies. |
| Policy | Parameters the society's governance may change within its constitution (tax rate, plan, work norms). |
| Tick / Cycle / Epoch | 1 hour / 24 ticks / 6 weeks. See §11.2. |
| Workplace | A production unit of a given type (Farm, Mine, …) occupying a land slot, holding machines, employing labor. |
| Organization (org) | The ownership-and-management wrapper around workplaces: firm, cooperative, collective, or state enterprise. |
| Household | A citizen's private sphere: pantry, dwelling, balance, standing plan. |
| Common Store | Society-wide stock of goods in moneyless systems; distribution by need. |
| State Store | Administered-price retail outlet in planned systems. |
| Standing Plan | A citizen's default behavior executed each tick in their absence. |
| Chronicle | Auto-generated feed of notable economic events in a society. |
| Observatory | Public cross-society comparison site. |
| Householder | An AI citizen following a published script. |
| Legacy firm | A starter org run by householders, for sale at book value. |

---

## 4. The Material Base

### 4.1 Commodity graph **[Proposed]**

Six goods in three tiers, plus dwellings. Chosen to be the smallest set that produces scarcity, specialization, trade, inequality, shortage, surplus, and a capital-accumulation decision.

| Tier | Good | Produced by | From | Consumed by |
|---|---|---|---|---|
| Raw | Grain | Farm | labor (+ land) | Mill |
| Raw | Ore | Mine | labor (+ land) | Foundry |
| Intermediate | Materials | Foundry | Ore | Workshop, Machine Shop, Builder |
| Consumer | Food | Mill | Grain | citizens (Food need) |
| Consumer | Wares | Workshop | Materials | citizens (Comfort need) |
| Capital | Machines | Machine Shop | Materials | workplaces (productivity) |
| Asset | Dwellings | Builder | Materials + labor | citizens (Shelter need) |

The critical tension is at the Foundry's output: Materials can become Wares (consumption now), Machines (productivity later), or Dwellings (shelter). That one allocation decision — consumption vs. accumulation — is made by different actors under different systems (individual firms chasing margins, a planning committee, an assembly vote, coops deciding retained earnings), and it's the single most ideologically loaded decision in economics. Making the graph funnel through it is deliberate.

**[Open]** A labor-only service good (Care: education/health, no material input, consumed for Comfort) is a strong v2 candidate — it's the one kind of good whose value is hardest to price and easiest to plan, which sharpens the contrast between systems. Held out of v1 for scope.

### 4.2 Needs **[Proposed]**

| Need | Meter decay | Satisfied by | Effect when low |
|---|---|---|---|
| Food | −4/tick (scaled by effort) | eating 1 Food/tick from pantry (auto) | output multiplier falls linearly below 50; hardship at <20 |
| Shelter | −2/tick if no dwelling; 0 if housed | occupying a dwelling slot | output ×0.7 unhoused; Comfort decays faster |
| Comfort | −1/tick | consuming Wares (1 per 6 ticks holds steady) | no output effect; drives wellbeing score and unlocks/locks aspirational options (§10) |

Food is the survival need (drives every-cycle demand, creates hardship). Shelter is the scarce-asset need (drives rents, allocation, housing politics). Comfort is the aspiration need (drives demand for discretionary goods, inequality in *lived* outcomes rather than just balances). Food and Shelter affect capacity; Comfort affects wellbeing only. That split matters: a system can keep everyone fed and housed and still leave them miserable, and the metrics (§14) should be able to say so.

Consumption is automatic from the household pantry per the Standing Plan; the player's decision is acquisition, not the act of eating. Pantry capacity is limited (e.g., 48 Food) so hoarding is possible but bounded — and visible.

### 4.3 Labor **[Proposed]**

Each citizen has a labor budget of 8 hours per cycle (recovers fully each cycle; reduced by fatigue debt after hardship). Hours are assigned to at most two workplaces. Output per hour at a workplace:

> output = base_rate(workplace type) × skill_mult(citizen, job family) × effort_mult × capital_mult(workplace)

- **Skill** per job family (0–100), grows ~logarithmically with hours worked, decays slowly when unused. skill_mult ranges 1.0 → 2.0. This creates specialization, switching costs, and a reason to keep a job.
- **Effort** ∈ {low ×0.6, normal ×1.0, high ×1.3}; multiplies Food decay by {0.8, 1.0, 1.3}; high effort for >2 consecutive cycles accrues fatigue debt (−1 hour next cycle).
- **Monitoring.** Output is metered exactly per workplace. Per-worker attribution is visible to the org's manager/planner with noise σ (an axis setting, §5 — a "monitoring intensity" parameter). Workers always see their own true output. This is what makes shirking a gamble rather than a free lunch, and what makes piece-rate vs. hourly vs. quota-bonus pay *different experiences*.

**[Open]** Self-reported output (with audit risk) as an alternative to metered output for planned systems — the classic fudge-the-numbers texture. Strong candidate for a v2 axis.

### 4.4 Workplaces, capital & land **[Proposed]**

Seven workplace types (Farm, Mine, Foundry, Mill, Workshop, Machine Shop, Builder). Each society has a fixed number of land slots per type (scarce natural endowment; e.g., 8 Farm, 6 Mine, unlimited for others but each costs Materials to found). Farms and Mines being slot-limited is what creates rents and the first fights over ownership.

Machines raise productivity with diminishing returns: capital_mult = 1 + 0.5·ln(1 + machines/worker). Machines depreciate 2%/cycle. Founding a workplace costs Materials (and money where it exists). This gives every system a genuine accumulation question — and a genuine "who decides" question.

### 4.5 Money & prices **[Proposed]**

Money, where it exists, is a society-local unit with a fixed initial stock (equal per-citizen endowment at join; new citizens are minted the same endowment so late joiners aren't structurally poor — note that this is mild inflation by design). No central bank in v1; credit is peer-to-peer contract. Price formation per the axis setting: continuous double auction order books per good (market), administered price lists (planned), or none (moneyless, common-store draws). Labor vouchers (non-transferable, expiring claims on the store) are an available money setting for lower-phase socialism variants.

---

## 5. The System Axes **[Proposed]**

An economy is a vector across these eight axes. Each setting is a concrete engine rule, not a slider on a vibe. Presets in §6 are named points in this space.

| # | Axis | Settings (each is an engine rule) |
|---|---|---|
| A1 | **Ownership of means of production** | `private` — workplaces, machines, and land slots are owned by citizens/firms via a share registry; freely transferable. `cooperative` — each org is owned equally by its current workers; non-transferable; leaving forfeits. `collective` — all owned by the society; no registry of owners; use governed by A8. |
| A2 | **Price formation** | `market` — order book per good; any party posts bids/asks. `administered` — a price list set by the policy authority; state stores sell at list; rationing on stock-out is queue-order (first request per tick) or ration-card (equal cap). `none` — no prices; goods flow via Common Store draws by need. |
| A3 | **Compensation** | `contract` — wage or piece-rate set per employment contract. `scale` — fixed grade table set by policy; grade by job family and skill band. `share` — org surplus divided among workers (equal or hours-weighted, org decides). `need` — no compensation; draw from Common Store by need. |
| A4 | **Labor allocation** | `free` — take any open position. `assigned` — authority assigns citizens to workplaces; transfer requests reviewed. `norm` — free choice under published work norms; contribution is public. |
| A5 | **Capital markets** | `open` — equity issuance, share trading, and peer credit contracts at any rate. `public-bank` — only a society-governed bank issues credit, by vote or formula. `none` — investment is a direct allocation of Materials to Machines by the org or society; no financial claims. |
| A6 | **Redistribution** | `none`. `tax-transfer(rate, brackets, floor)` — income tax funds a treasury that pays a need floor and public goods. `provision` — the society guarantees Food and Shelter in kind regardless of income. `total` — all output pooled; no private income (implied by A3=need). |
| A7 | **Monitoring intensity** | `high` — per-worker output attribution exact. `medium` — noise σ=0.25. `low` — σ=0.6 (only workplace totals are reliable). Determines how well managers/planners/co-workers can see who's carrying whom. |
| A8 | **Economic governance** | `none` — rules fixed; no offices, no votes. `direct` — assembly; any citizen proposes; majority decides; offices are short-term, rotating, recallable. `representative` — elected body for a term sets policy. `committee` — small elected planning body with full plan authority between elections; recall by supermajority. |

**Money** is derived from A2/A3: `market` or `administered` pricing implies money; `none` pricing implies no money (or vouchers, a documented variant).

**Transfers** (gifting goods or money to another citizen) are a *primitive*, always available, never an axis. Every informal economy in history is built on them; restricting them is how a black market becomes visible rather than impossible.

---

## 6. Shipping Presets **[Proposed]**

### 6.0 Preset matrix

| Preset | A1 Own | A2 Price | A3 Pay | A4 Labor | A5 Capital | A6 Redist | A7 Monitor | A8 Gov |
|---|---|---|---|---|---|---|---|---|
| Pure Capitalism | private | market | contract | free | open | none | high | none |
| Pure Communism | collective | none | need | norm | none | total | low | direct |
| Central Planning | collective | administered | scale | assigned | none | provision | medium | committee |
| Social Democracy | private | market | contract | free | open | tax-transfer | high | representative |
| Market Socialism | cooperative | market | share | free | public-bank | provision (basic) | medium | representative |

### 6.1 Pure Capitalism — "Freeport"

*Ideology being tested:* voluntary exchange between property owners, with no coercion beyond contract enforcement, produces prosperity and rewards contribution.

- **Start.** Every citizen joins with an equal money endowment and nothing else. Land slots are unclaimed; founding a workplace on one requires a Materials cost and the founder becomes 100% owner (homesteading). Legacy firms run by householders exist so there are jobs on day one; they're for sale at book value.
- **Firms.** Any citizen with capital founds a firm: a share registry, a treasury, one or more workplaces, a manager (owner-appointed; initially the founder). The manager sets prices (posts asks), wages (posts job offers: hourly or piece-rate, term, notice period), production mix, and investment (buy Machines). Profit accrues to the treasury; dividends are declared by the majority owner; shares trade on the order book like any good.
- **Labor.** Free. Employment is a contract; either party may terminate with the contracted notice. Unemployment is possible and has no cushion.
- **Credit.** Any citizen or firm may lend to any other by contract (principal, rate, term, collateral optional). Default = collateral seized, then a public default flag. No bankruptcy protection in v1.
- **Housing.** Dwellings are property; rent or buy by contract. Unhoused citizens take the Shelter penalty.
- **Governance.** None. There is nothing to vote on. Charity, mutual aid, unions, cartels — all possible via contracts and transfers, none provided.
- **Scoreboard.** Net worth (balance + holdings at market), firm valuation, and a "self-made" track (net worth excluding endowment).
- **The felt experience the design is aiming at:** freedom and precarity at once. Your day is prices. Nobody owes you anything, and anything is for sale.

### 6.2 Pure Communism — "The Commune"

*Ideology being tested:* free producers, collectively owning what they make, will contribute according to ability and draw according to need without money, markets, or a coercive state.

- **Start.** All workplaces (a seeded set on the land slots) belong to the society. No money exists. Each citizen has a household pantry and a dwelling assigned from the collective stock.
- **Production and the Common Store.** All output flows into the Common Store. Citizens draw Food, Wares, and dwelling assignments from it. Draw entitlement each tick = whatever brings the citizen's need meters to full; surplus beyond that is distributed as equal shares. When the store is short, rationing is **need-first** (largest shortfall served first) — "to each according to need," made concrete. The assembly can vote to change the rationing rule (a policy choice, and a telling one).
- **Labor.** Free choice of workplace under published **work norms** (the assembly sets suggested hours per citizen per cycle, default 6). No enforcement. Every citizen's contribution is public in the Ledger of Contribution — hours exactly (they're a decision), output as attributed under the society's monitoring setting (low by default: there's no foreman). Whether to meter output exactly is a *policy* the assembly can vote in; a commune that chooses surveillance of its own members has told us something. Coordinators publish a Plan of targets per workplace; it's advisory.
- **Investment.** The assembly votes each cycle on the Materials split: Wares / Machines / Dwellings. This is the accumulation decision, collectivized.
- **Governance.** Direct assembly. Any citizen may propose; simple majority; votes close each cycle. Coordinators (3) are elected for 5-cycle terms, no consecutive terms, recallable by majority at any time. Coordinator powers: publish the Plan, open/close workplaces on the land slots, propose the rationing rule. They cannot assign labor, cannot alter draws, cannot punish.
- **Transfers.** Goods may be gifted from pantry to pantry. (Whether a barter economy or favor economy emerges around scarce Wares is one of the things we're watching.)
- **Scoreboard.** Standing: a public contribution record, assembly honors (a proposal type), and the society-level need-fulfillment rate — the commune succeeds *together* or not at all. No individual net worth exists to display.
- **Felt experience aimed at:** security and dependency at once. Your day is the Store's stock levels and the Ledger. Nobody can fire you; nobody can be made to work; everyone can see what you did.

### 6.3 Central Planning — "The Directorate"

*Ideology being tested:* a state that owns production and plans it scientifically can meet needs and grow faster than uncoordinated markets.

- **Start.** All workplaces state-owned. Money exists (wages, state-store prices). Citizens are assigned to workplaces by the Planning Committee (initially by algorithm balancing the Plan).
- **The Plan.** Each cycle the Committee publishes: output targets per workplace, the Materials split, the price list, and the wage-grade table. Workplaces that hit target pay a **plan bonus** to their workers (default +25% of wage). Overfulfillment raises next cycle's target (the ratchet is a policy option, on by default — it's authentic).
- **Distribution.** State Store sells at administered prices, stock permitting. On stock-out, queue-order per tick; the Committee may impose ration cards (equal cap per citizen) on any good. Provision: every citizen is guaranteed a dwelling assignment and a minimum Food ration at zero price — the safety net is real.
- **Labor.** Assigned. Citizens may file a transfer request; the Committee approves or denies. Unemployment doesn't exist; underemployment does.
- **Capital.** None. Investment is the Committee's Materials split.
- **Governance.** Committee of 3, elected by all citizens every 10 cycles, full plan authority in between, recall by two-thirds. Committee members are paid from the wage-grade table they themselves publish — the engine grants them no allocation of its own, so any privilege of office is a choice the Committee makes in public (and the one everyone will watch).
- **Transfers.** Money and goods may be transferred person to person. There is no order book. (The informal economy — reselling scarce Wares at a markup via DM — is not prohibited by the engine; whether the Committee tries to police it via ration policy is up to them.)
- **Scoreboard.** Rank/grade, plan-fulfillment percentage (personal and workplace), honors conferred by the Committee.
- **Felt experience aimed at:** order and shortage. Your day is your quota, the Store's stock, and the queue.

### 6.4 Social Democracy — "The Republic"

*Ideology being tested:* markets allocate well but distribute badly; democratic taxation and provision can keep the former and fix the latter.

- Everything from Pure Capitalism, plus:
- **Legislature** of 5, elected every 10 cycles, sets policy by majority: income tax rate and brackets, need floor (Food and Shelter provision paid from the treasury), minimum wage, public dwellings, and whether to fund a public investment bank.
- **Unions.** A collective-bargaining primitive: workers of a firm may form a union org that negotiates a single contract for all members; strike = coordinated hours-zero with union strike pay from dues.
- **Scoreboard.** Net worth *and* a wellbeing index — the republic advertises both, which is its argument.
- **Felt experience aimed at:** cushioned markets. Your day is prices, the tax bill, and the election.

### 6.5 Market Socialism — "The Cooperative Commonwealth"

*Ideology being tested:* the problem with capitalism is who owns the firm, not the market; worker ownership plus markets gets efficiency without exploitation.

- **Ownership.** Every org is a cooperative: one worker, one vote; the coop elects a manager; surplus is shared (coop chooses equal or hours-weighted); leaving forfeits the share. Coops admit new members by vote.
- **Markets.** Goods and labor-membership are free; prices on order books.
- **Capital.** No private capital market. A Public Investment Bank, funded by a capital levy on coops (a small % of machine value per cycle), lends to new or expanding coops; the legislature sets the levy and the lending rule (formula or vote).
- **Redistribution.** Basic provision of Food; dwellings by market.
- **Governance.** Representative (legislature of 5) at the society level; direct at the coop level.
- **Scoreboard.** Your coop's surplus per member, your membership tenure, coop honors.
- **Felt experience aimed at:** owner and worker at once. Your day is your coop's books and the vote on whether to admit the new applicant.

### 6.6 Held for v2

State Capitalism (Central Planning with `market` pricing and `contract` pay — the one-axis flip toward Dengist reform), Anarcho-syndicalism (Pure Communism with `cooperative` ownership and federated coop assemblies), Georgist Capitalism (Pure Capitalism with land rent socialized), Lower-phase Socialism (Pure Communism with labor vouchers).

---

## 7. Organizations, Contracts & Transfers **[Proposed]**

These are the primitives from which everything in §16 is meant to emerge. The rule: provide few, general primitives; never script the outcome.

### 7.1 Organizations

An org has: a type (firm / cooperative / collective / state enterprise / union / association), a membership or share registry per its type, a treasury (where money exists), zero or more workplaces, an internal channel, and a manager role whose selection rule follows the org type (owner-appointed, member-elected, assembly-elected, committee-appointed). Associations are the catch-all: any group of citizens may form one in any system (a mutual-aid society, a cartel, a party, a book club) — it has a channel, a treasury or pooled pantry, and member-voted rules on disbursement. Whether associations become unions, charities, or mafias is not ours to decide.

### 7.2 Contract types

| Contract | Parties | Terms | Enforcement |
|---|---|---|---|
| Employment | org ↔ citizen | hourly wage or piece-rate, hours, term, notice | pay auto-debited each cycle; breach = notice-period pay |
| Sale (order book) | any ↔ any | good, qty, price | escrow; atomic settlement at tick |
| Sale (direct) | any ↔ any | good, qty, price | escrow; used where no order book exists |
| Credit | any ↔ any | principal, rate, term, collateral | auto-repayment; default seizes collateral, then public flag |
| Lease | owner ↔ citizen/org | dwelling or workplace, rent, term | auto-debit; non-payment ends lease after grace |
| Share issue/transfer | firm ↔ any | shares, price | registry update |
| Collective agreement | union ↔ org | wage floor, hours, term | overrides individual contracts for members |
| Pledge | citizen ↔ org/assembly | hours or goods committed, term | non-enforced; recorded publicly (moneyless systems) |

Each preset enables a subset (there are no credit contracts in the Commune; there are no employment contracts in the Directorate, only assignments). The engine, not the players, enforces every enabled contract — the "night-watchman" is the code.

### 7.3 Transfers

Any citizen may transfer goods from pantry or money from balance to any other citizen or org, at any time, in any system, with an optional memo. Transfers are logged. That's the whole rule. Gifts, bribes, dues, alms, and side-payments are all just transfers with different memos, and the telemetry can tell them apart by pattern afterward.

---

## 8. Governance & Special Roles **[Proposed]**

### 8.1 Governance primitives

Proposals (typed: policy-parameter change, office election, recall, honor, resolution/free text), ballots (one citizen one vote by default; per-share in firm votes; per-member in coops), terms, quorum (default: 20% of active humans), and the closing rule (votes close at cycle end). A society's constitution enables a proposal-type subset and names who may propose (anyone / office-holders only). Householders never vote and never hold office.

### 8.2 Role catalog

| Role | Systems | Filled by | Powers | Removal |
|---|---|---|---|---|
| Founder/Owner | Cap, SocDem | self, by capital | appoint manager, declare dividends, sell | sells out |
| Manager | all with orgs | per org type | prices, hiring, production mix, investment | owner/members |
| Coordinator ×3 | Commune | assembly election, 5-cycle term, no consecutive terms | publish plan, open/close workplaces, propose rationing rule | recall, simple majority, any time |
| Planning Committee ×3 | Directorate | election, 10-cycle term | full plan authority, assignments, prices, ration cards | recall, two-thirds |
| Legislator ×5 | SocDem, MarketSoc | election, 10-cycle term | policy parameters by majority | recall, simple majority |
| Union steward | SocDem | member election | negotiate collective agreement, call strike | members |
| Bank board ×3 | MarketSoc | election | lending decisions per rule | recall |

### 8.3 Absence and vacancy

An office-holder absent (no session) for 3 cycles is auto-vacated and an election opens. Until filled, the last plan/policy stands. Elections with no candidates re-run each cycle; a society with a required role unfilled for 5 cycles gets a Chronicle headline ("The Directorate has no Committee") and the engine falls back to "last plan continues" indefinitely. It never appoints an AI.

### 8.4 Power and its abuse

Within the rules: it's data. The Committee that assigns itself the best dwellings, the owner who fires organizers, the assembly that votes to cut a shirker's draw (if the rationing rule allows) — all recorded, all legitimate targets for in-system recourse. Outside the rules: no role can touch constitutional settings, telemetry, identities, private messages, or another citizen's account. Platform-level abuse (harassment, doxxing, real-money trading) is moderation, §13.

---

## 9. Player Experience

### 9.1 The core loop **[Fixed, elaborated]**

One session (5–20 min), regardless of system:

1. **Situation.** Home screen: your household (need meters, pantry, dwelling, balance or draw entitlement), your labor status, and the society's headline stats and Chronicle.
2. **Obligation / work.** Confirm or adjust your labor allocation and effort for the cycle. Respond to what the system asks of you (a quota, a work norm, a shift offer, an assignment).
3. **Compensation.** See what you got and why — a payslip, a store draw record, a plan bonus, a coop share. Every number links to the rule that produced it.
4. **Consumption.** Confirm the standing plan is feeding and housing you; buy/draw/queue for Wares if you want Comfort.
5. **Surplus.** Do something with what's left over, as the system permits: save, invest, lend, found, contribute, pledge, gift.
6. **Society.** Read the Chronicle, check the dashboards, vote if there's a ballot, negotiate if there's an offer, talk.

The loop is identical in shape across systems and radically different in content — that identity-of-shape is what makes the difference feel like the *system's* doing rather than the game's.

### 9.2 Session archetypes **[Proposed]**

| Archetype | Length | What it looks like | What the design must guarantee |
|---|---|---|---|
| Check-in | 3–10 min, daily | read payday/ration, adjust plan, vote, close | one screen shows everything that changed; one tap to keep the plan |
| Evening | 30–90 min | trade, negotiate, reorganize, campaign, found | several ticks resolve during the session; enough live counterparties (AI fill) |
| Office | varies | planning, managing, legislating | tooling for the role is a first-class UI, not a form |
| Spectator | any | observatory, chronicle, other societies | public read-only views require no citizenship |

### 9.3 The Standing Plan **[Proposed]**

Every citizen's plan has: labor allocation (workplace, hours, effort — or "accept assignment"), consumption rules (keep pantry ≥ N Food; buy Wares when Comfort < M and balance > B), saving rule (keep balance ≥ S), standing bids/asks, and a vote default (abstain / follow a named citizen / no default). The plan executes each tick. It's fully editable, and the UI's first job at every session is to show what the plan did since you left. After 7 cycles of absence the citizen goes **dormant**: stops producing and consuming, assets frozen (rent stops, wages stop, dwelling released in collective systems), excluded from population stats, returns on next login with a "while you were away" digest. Presence matters because the plan can't *react*; absence isn't ruinous because it can *persist*.

### 9.4 Onboarding — "working within two minutes" **[Proposed]**

Join a society → pseudonymous handle → the society's **Welcome Brief**, written in the society's own voice and vocabulary (a firm's job board vs. a Committee assignment letter vs. an assembly greeting), which is also the tutorial → first labor decision (choose a legacy-firm job / receive your assignment / pick a workplace under the norm) → standing plan pre-filled with sane defaults → home screen. First cycle is a guided read of the payslip/draw record. No tutorial mode; the interface teaches by being legible.

### 9.5 Longer arcs **[Fixed, elaborated]**

Specialization (skill in a job family), career (worker → manager/steward/coordinator), accumulation or standing, founding, organizing, holding office, and the society-scale stories the Chronicle turns into narrative: the first bankruptcy, the plan that fell short, the strike, the reform vote, the hoarding scandal, the epoch's closing statements.

---

## 10. Success, Progression & Scoreboards **[Proposed]**

Per the pillar, no universal victory metric. Each society's home screen shows *its own* scoreboard, derived from its constitution:

| System | What "doing well" is shown as | Why that's the honest choice |
|---|---|---|
| Pure Capitalism | net worth, firm valuation, self-made delta | the system's own claim is that wealth measures contribution |
| Pure Communism | contribution record, honors, society need-fulfillment | there's no individual wealth; the claim is collective flourishing |
| Central Planning | grade/rank, plan fulfillment %, honors | the claim is that fulfilling the plan is the contribution |
| Social Democracy | net worth *and* wellbeing index | the claim is you can have both |
| Market Socialism | coop surplus per member, tenure | the claim is that ownership by workers is the point |

Cross-society comparison is exiled to the Observatory (§14), where it uses common metrics, framed as invitations to interpret.

Progression that persists across societies and epochs (cosmetic and reputational only): a citizen profile with epochs lived, roles held, honors, and a "biography" line the player writes. No material carry-over, ever.

---

## 11. Multiplayer Structure

### 11.1 Societies **[Fixed, elaborated]**

One system per society. Two classes: **canonical** (fixed constitution, verified identity, standard communication set, epoch-based, included in the Observatory comparison) and **community** (any preset or custom vector, may amend, may be permanent, listed but labeled). A person may hold citizenship in multiple societies; the profile shows all of them.

### 11.2 Time **[Proposed — see Q6]**

Tick 1 h · Cycle 24 ticks · Epoch 42 cycles (6 weeks). Production, consumption, order-book clearing, and need decay resolve at each tick. Payroll, plan bonuses, ration resets, tax, vote closes, and the Chronicle's daily edition resolve at cycle end. A society's cycle boundary is fixed at creation (e.g., 04:00 in the founding timezone) so "payday" is a shared moment.

### 11.3 Population & AI householders **[Proposed — see Q5]**

Floor 40, comfortable 100–300, cap 500; a canonical preset spawns a new society when the previous one passes 300 active humans. Householder script is published in-game (transparency is part of neutrality: players should know the AI isn't secretly tilting the economy). Householders: take best available offer / assignment; normal effort; buy needs; save 10%; sell surplus at last price; no office, no vote, no founding, no lending. Legacy firms (market systems only): cost-plus pricing at 15% markup, hire at the median offered wage, for sale at book value to any citizen. Householders emigrate (dormant, assets liquidated to the treasury or store) as humans arrive.

### 11.4 Player-run agents **[Proposed — see Q5]**

Public API = the same API the web client uses. API keys are per citizen. Telemetry records the share of actions via API key. `agents_allowed` is a society flag (canonical default: allowed, recorded; a parallel "unassisted" canonical track is a v2 option once population supports it). Agent-only strategy leagues: v2, separate track.

### 11.5 Epochs **[Proposed — see Q7]**

Epoch end sequence: cycle 40 announcement → cycle 42 final resolution → Chronicle compiled → Observatory snapshot frozen → closing statements window (48 h) → archive (read-only, public) → new epoch spawns with the same constitution, fresh material state, and the same citizen roster invited back. Early end: active humans < floor for 5 cycles ("collapse"), or a constitutional emergency in a community society.

---

## 12. Communication **[Proposed — see Q8]**

| Channel | Scope | Structured? | Purpose |
|---|---|---|---|
| Square | society-wide free text | no | public talk |
| Assembly floor | society-wide, tied to proposals | semi (threads per proposal) | deliberation with a record |
| Org channel | members of an org | no | coordination |
| DM | 1:1 | no | the informal economy lives here |
| Notice board | society-wide | yes (typed ads: job, sale, wanted, credit, membership) | the market's front page |
| Chronicle | society-wide, read-only | generated | the shared world |

Chronicle events (examples): price moves >10%, stock-outs, plan published/fulfilled/missed, elections, recalls, foundings, bankruptcies, strikes, largest transfers, hardship counts. Written in the society's voice.

Constraints: Discord is expected and not fought. All in-game comms are logged and players are told so. Communication set is a constitutional field, standard in canonical societies; restricted variants are a v2 experimental manipulation.

---

## 13. Identity, Alts, Trust & Safety **[Proposed — see Q9]**

- Pseudonymous persistent handle; email-verified account minimum; canonical societies require a second verification signal (TDD to choose).
- One citizen per person per society, enforced by account; alt detection by telemetry pattern; flagged citizens excluded from research data, then moderated.
- Research consent at signup: aggregate behavioral study, pseudonymous, no sale of data, comms logged; plain-language, one screen.
- Moderation covers harassment, doxxing, real-money trading, and platform exploits — not in-system economic behavior, however ruthless.
- No monetization in v1 (non-goal), which also removes pay-to-win as a neutrality threat.

---

## 14. Measurement, Observatory & Telemetry **[Proposed]**

### 14.1 Within a society (live, public, system-appropriate)

Market systems: price indices, wage distribution, unemployment, firm count, credit outstanding. Planned: plan fulfillment by workplace, store stock levels, queue lengths, ration status. Moneyless: store stock, contribution distribution, draw fulfillment. All systems: need-fulfillment rate, hardship count, wellbeing index distribution, population and activity.

### 14.2 Across societies — the Observatory

Common metrics must be *definable in every system*, which rules out anything denominated in money or wealth. The comparable set:

- **Real output** — goods produced, weighted by a fixed reference basket (same weights everywhere).
- **Median wellbeing** — the composite need-satisfaction index over the cycle.
- **Need-fulfillment rate** — share of citizen-cycles with all needs above hardship.
- **Consumption inequality** — Gini of per-citizen consumption (Food + Wares + housed), *not* wealth. Consumption is defined everywhere; wealth isn't. This is the key trick that makes the poles comparable.
- **Participation & retention** — active humans, session frequency, epoch completion rate.
- **Mobility** — rank change in consumption decile over the epoch.
- **Investment share** — Materials → Machines as a share of Materials output (the accumulation decision, compared).

Framing: side-by-side, with each society's own scoreboard shown alongside, never a ranking. Copy on the Observatory says what each metric can and can't tell you.

### 14.3 Telemetry

Every action is an event (actor, society, tick, type, payload, client kind). Every tick emits a state snapshot for aggregates. Exportable per epoch as a pseudonymized dataset. Retention and access policy is a TDD item; the design commitment is that the dataset is complete enough to reconstruct any citizen's economic life and any society's history.

---

## 15. Interface Design — the UI Embodies the System **[Proposed]**

Same engine, same component library, different *lived interface*. Three levers:

**Vocabulary.** A per-preset lexicon rewrites every label. Examples:

| Concept | Freeport | Commune | Directorate | Republic | Commonwealth |
|---|---|---|---|---|---|
| Compensation | Payslip | Draw record | Wage & bonus | Payslip (after tax) | Member share |
| Job | Position | Contribution | Assignment | Position | Membership |
| Home screen title | Your Accounts | Your Household & the Store | Your Assignment & the Plan | Your Accounts & the Republic | Your Coop |
| Society stat | Price index | Store stock | Plan progress | Price index & treasury | Coop league |

**Information.** What is *shown* differs by system because what *exists* differs. Freeport's home shows prices, balance, open contracts — no plan progress, because there is no plan. The Directorate's shows quota, Store stock, queue position — no price chart, because prices don't move. The Commune's shows Store stock, the Ledger of Contribution, and tonight's ballots — no balance, because there's no money. This is the concept doc's §8.3 made literal: the absence of a widget is a design statement.

**Tone.** Chronicle voice, Welcome Brief voice, and empty-state copy differ per society and are written to be *loved by a partisan*, not to wink at one. Neutrality review (Appendix B) covers copy, not just rules.

Fixed UI conventions everywhere: every number has an "explain" affordance showing the rule and inputs (legible state); the Standing Plan is one screen; the Chronicle is always one tap away; role tooling (plan editor, legislature ballot builder, coop books) is built as a proper workspace, since office-holders are the players who spend the longest sessions.

---

## 16. Expected Emergence **[Proposed]**

What we hope to see, and which primitive permits it. If a row lacks a primitive, that's a design gap.

| Phenomenon | Systems where expected | Enabled by |
|---|---|---|
| Firms, hiring, wage competition | Cap, SocDem | orgs, employment contracts, order books |
| Monopoly / cartel on land goods | Cap | slot-limited Farms/Mines, associations, DMs |
| Charity, mutual aid | all | transfers, associations |
| Unions, strikes | SocDem (built), Cap (emergent via associations) | collective agreement, associations, hours-zero |
| Shirking and its detection | all | effort choice, monitoring axis, public ledger |
| Free-riding on the commune | Commune | norm-based labor, public ledger |
| Ostracism / social enforcement | Commune | assembly resolutions, honors, rationing rule votes |
| Queues and hoarding | Directorate, Commune | stock-outs, pantry caps, ration cards |
| Black market / informal resale | Directorate | transfers + DMs with no order book |
| Quota gaming, the ratchet | Directorate | plan bonus, ratchet policy |
| Privilege of office | Directorate, Commune | committee allocations, coordinator visibility |
| Reform movements | SocDem, MarketSoc, community societies | proposals, elections, amendments |
| Credit booms, defaults | Cap, SocDem | credit contracts, collateral |
| Coop admission gatekeeping | MarketSoc | membership votes, share forfeiture |
| Inequality of consumption | all (magnitudes differ) | measured, not designed |

---

## 17. Balance & Tuning Approach **[Proposed]**

1. **Headless simulation first.** Before any human plays, run each preset with householders only, for simulated epochs, to find constants (base rates, decay, endowments, slot counts) at which the economy neither starves nor saturates. A householder-only society should be *boring and stable*; interesting things should require humans.
2. **Tuning targets** (per preset, householders only): need-fulfillment ≥ 95%, Materials output roughly balanced across the three sinks, no good with a persistent stock-out, prices (where they exist) within ±30% of a reference basket over an epoch.
3. **Never tune for fun; tune for stability, then let humans make it fun.** Where a preset is stable and dull under humans, the fix is texture from the system's own logic (§1 pillar), sourced from the emergence table — not new mechanics.
4. **Neutrality regression.** Each tuning change is checked against Appendix B; a change that helps one preset's fulfillment rate more than another's is flagged for review, not rejected — but it's noticed.
5. **Parameter table** (Appendix A) is the single source of tunables; all live in config, none in code.

---

## 18. Scope, MVP & Prototype Plan **[Proposed]**

| Phase | Goal | Content | Exit criterion |
|---|---|---|---|
| 0. Sim | tuning + engine validation | tick engine, 6 goods, 7 workplaces, needs, labor, householders, all 5 preset configs (no UI) | 5 stable householder epochs per preset |
| 1. Freeport prototype | first humans | Pure Capitalism, firms, order books, employment/credit/lease contracts, standing plan, Chronicle, basic stats, API | 10–20 humans, 1-week epoch, retention & interviews |
| 2. Commune prototype | the contrast | Pure Communism, Common Store, assembly, coordinators, Ledger | same cohort lives a week here; the "felt difference" interview |
| 3. Observatory + Republic | comparison and the middle | Social Democracy, legislature, unions, Observatory v1, epoch archive | first public cross-society page |
| 4. Directorate + Commonwealth | full v1 set | Central Planning, Market Socialism, community societies, agents flag | five canonical societies live |

v1 non-goals restated: no world, no war, no inter-society trade, no monetization, no mobile-native app, no historical scenarios, no Care good, no self-reported output, no restricted-comms variants.

---

## 19. Risks **[Proposed]**

| Risk | Why it's real | Mitigation |
|---|---|---|
| Dull equilibrium | a stable economy with few humans is a spreadsheet | Chronicle, role tooling, AI fill taper, epoch pacing; interview for "what did you feel" not "was it fun" |
| Population too small for a market | thin markets make capitalism feel like a shop | AI fill floor, legacy firms, one society per preset until 300 |
| Tilt creep | every tuning pass is a chance to favor a system | Appendix B on every change; partisan reviewers for each preset |
| Ideological flame wars | the topic attracts them | in-game comms moderated for conduct not content; Observatory copy avoids verdicts |
| Griefing office-holders | a bad planner can wreck a cycle | recall, term limits, vacancy fallback; and it's data |
| Agent dominance | one good bot outtrades everyone | recorded, flaggable per society, unassisted track later |
| Research consent & privacy | it's a study | plain consent, pseudonymity, aggregate publication only |
| Solo-dev scope | five presets, governance, comms, observatory | phase gates; Phase 1 is one preset, no governance, no observatory |

---

## 20. Remaining Open Questions

1. Endowment at start in market systems: equal money (proposed) vs. lottery vs. auctioned land — equal is neutral but arguably un-capitalist; decide after Phase 1 interviews.
2. Second identity signal for canonical societies (TDD).
3. Whether Comfort should feed back into labor capacity at all (currently no) — risk that Comfort is ignorable.
4. Ration rule vocabulary for the Commune: need-first vs. equal-shortfall vs. lottery; ship one, let the assembly change it, or ship the vote?
5. Bankruptcy/debt exit in Freeport: none in v1 is authentic but may be a churn cliff; watch it.
6. Care good and self-reported output — v2 axis candidates, revisit after Phase 2.
7. Whether the Directorate's Committee is elected (proposed) or seeded by the engine for the first term — affects the first-week experience heavily.
8. Cycle boundary time for a society with a global population.
9. Observatory reference basket weights — who sets them and how to justify neutrality.
10. Whether cross-epoch reputation should be visible in-society (it may create a class of "notables" that contaminates fresh starts).
11. Monitoring (A7) as a constitutional default vs. an in-system choice: an org-level purchasable upgrade in market systems, an assembly policy in the Commune (as §6.2 now proposes). Making it a choice everywhere is more neutral and more interesting; it also adds a mechanic. Lean: choice everywhere, defaults per preset.

---

## Appendix A. Parameter Table (starting values, all tunable)

| Parameter | Value | Notes |
|---|---|---|
| Tick / cycle / epoch | 1 h / 24 ticks / 42 cycles | §11.2 |
| Labor budget | 8 h/cycle | max 2 workplaces |
| Effort multipliers (output) | 0.6 / 1.0 / 1.3 | low / normal / high |
| Effort multipliers (Food decay) | 0.8 / 1.0 / 1.3 | |
| Fatigue debt | −1 h/cycle after 2 consecutive high-effort cycles | |
| Skill range / growth | mult 1.0–2.0; +1 skill per ~log(hours) | decay −1/10 idle cycles |
| Food decay | −4/tick; eat 1 Food/tick | pantry cap 48 |
| Shelter decay | −2/tick unhoused | |
| Comfort decay | −1/tick; 1 Wares per 6 ticks holds | |
| Hardship threshold | Food < 20 for a full cycle | flag + −2 h budget |
| Destitution | hardship 3+ cycles | output floor ×0.25; options narrowed |
| Output floor (low Food/Shelter) | ×0.4 | |
| Capital multiplier | 1 + 0.5·ln(1 + machines/worker) | |
| Machine depreciation | 2%/cycle | |
| Land slots | Farm 8, Mine 6, others unlimited | per society |
| Founding cost | 20 Materials (+ money where exists) | |
| Population floor / comfortable / cap | 40 / 100–300 / 500 | |
| Householder script | best offer, normal effort, save 10% | published |
| Legacy firm markup | 15% | for sale at book |
| Dormancy | 7 absent cycles | |
| Office vacancy | 3 absent cycles | |
| Quorum | 20% active humans | |
| Plan bonus | +25% wage | Directorate |
| Ratchet | on | Directorate |
| Coordinator term | 5 cycles, no consecutive | Commune |
| Committee / legislature term | 10 cycles | |
| Work norm default | 6 h/cycle | Commune |
| Monitoring σ | 0 / 0.25 / 0.6 | high / medium / low |
| Collapse | active humans < floor for 5 cycles | |

## Appendix B. Neutrality Review Checklist

Apply to every preset, every tuning change, every piece of copy.

1. Would a committed advocate of this system say the rules are a fair implementation of what they believe? Name the advocate (a Hayekian, a Marxist, a Nordic social democrat, a Yugoslav self-management theorist, a Soviet planner) and answer as them.
2. Is any hardship in this system the *system's own* consequence, or something the engine added? (Unemployment in Freeport: the system's own. Shortage in the Directorate: the system's own. A random "corruption event": added — forbidden.)
3. Is any comfort in this system the *system's own* provision, or a gift from the engine? (The Directorate's ration floor: its own. A Freeport safety net: a gift — forbidden.)
4. Does the vocabulary and Chronicle voice describe the system the way its adherents describe it, without irony?
5. Does the scoreboard measure what the system claims to value?
6. Does the tuning change move one preset's headline metrics more than another's? If yes, is that because of the system's logic or because of our constants?
7. Would the same player, reading only the interface, be able to tell you what this society believes without being told?

---

*End of GDD v0.1. Next: review the [Proposed] decisions in §2 and §6, then the TDD.*
