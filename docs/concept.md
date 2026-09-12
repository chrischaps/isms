# Concept Doc: *Isms* (working title)

*A web-based multiplayer economic simulation exploring how people behave under different economic systems.*

**Status:** Initial concept · **Author:** Chris · **Date:** Sept 2026
**Purpose of this doc:** Establish the vision, core concept, and open questions well enough to drive a future Game Design Doc (GDD) and Technical Design Doc (TDD).

---

## 1. Pitch

Players join a persistent online economy and simply *live* in it: they work, get paid, consume, save, and (where the rules allow) trade, invest, or organize. The twist is that each server runs a different economic system — pure capitalism at one end, pure communism at the other, and configurable variants along the spectrum between them. The game itself editorializes nothing; the rules of each system are implemented faithfully, and everything interesting emerges from how players actually behave inside them.

The entire experience is rendered as pure UI — buttons, sliders, graphs, tables, text. No avatars, no map, no world. The economy *is* the game.

## 2. Vision & Goals

**North star:** This is a game first — an entertainment product. But its entertainment value is of a specific kind: **the fun is feeling how the system shapes your experience as a participant.** What you're free to do and what you're compelled to do, what you can aspire to, what you depend on others for, what a good day looks like — these should differ palpably between a capitalist society and a communist one, and that felt difference *is* the product. A player who has lived a week in each should come away with a visceral, first-person sense of both.

The other purposes are real but derivative — they're served *through* the north star, not traded against it:

- **Social experiment:** if the systems are authentic and the players are real, honest behavioral observation falls out for free.
- **Education:** living inside a faithfully implemented system teaches it more durably than reading about it — but only if the implementation is faithful.

**Design principles that follow from the north star:**

- **Authenticity is the fun.** Never tilt a system to be "the fun one" or "the broken one" — tilt destroys the very thing being sold. If a system feels dull, the fix is to surface its *inherent* texture (scarcity politics, queue culture, negotiation, informal favor economies), never to graft on foreign mechanics.
- **Felt experience beats abstract accuracy.** When model fidelity and player feel conflict, prefer the design that makes the system's character legible in play. Simplify the economics; never simplify away the ideology.
- **Fidelity to the idea, not to history.** "Pure communism" means the system as theorized (collective ownership, distribution by need), not a caricature of any historical regime. Same for pure capitalism. Variants model real-world hybrids.
- **Fun through agency, not through tilt.** Every system stays engaging by giving players meaningful choices *within* its rules (which job, what to consume, whom to cooperate with, whether to comply).
- **Every system defines its own success.** Don't impose a single victory metric (e.g., personal net worth) — that itself encodes an ideology. Each system surfaces the scoreboard its own logic implies; cross-system comparison happens at the societal level (see §7).

## 3. Core Concept

- **Web-based**, playable in any browser, low friction to join. Maximizes participation (which the experiment goal needs) and keeps scope tight (which solo development needs).
- **Pure UI representation.** All state and actions expressed via dashboards, forms, charts, and feeds. This is a deliberate aesthetic and scope decision, in the spirit of Taskman rather than The Capitol.
- **One system per server.** Each server (a "society") is configured with a single economic system at creation. Players can hold citizenship in multiple societies, but each society's economy is closed and internally consistent.
- **Persistent, drop-in/drop-out.** Societies run continuously. The design must make short, occasional sessions viable and must handle absence gracefully (see §6).

## 4. The Economic Model

### 4.1 Systems as configurations, not hardcoded modes

Rather than implementing "capitalism" and "communism" as separate codepaths, define an economy as a **configuration across a set of axes**. Named systems are presets; the spectrum between them is the parameter space. Candidate axes:

| Axis | One pole | Other pole |
|---|---|---|
| Ownership of means of production | Fully private | Fully collective/state |
| Price formation | Free market | Centrally planned/fixed |
| Wage determination | Market/negotiated | Equal or need-based |
| Labor allocation | Free job choice | Assigned/quota-based |
| Capital & investment | Open capital markets | None |
| Redistribution | None | Total |
| Governance of economic policy | None (rules are fixed) | Democratic / central planner |

**Presets** might include: Pure Capitalism, Pure Communism, Social Democracy, Market Socialism, State Capitalism, Anarcho-syndicalist commune, etc. The preset list and the exact axes are a core GDD work item — each needs a defensible operationalization (this is where the educational goal demands rigor).

This parameterization is also what makes the project *research-shaped*: it turns "capitalism vs. communism" into "which parameters drive which behaviors."

### 4.2 The material base

Keep the simulated economy as simple as possible while still supporting the phenomena we want to observe (scarcity, specialization, trade, inequality, shortage, surplus):

- A small commodity graph — e.g., raw resources → intermediate goods → consumer goods. Two or three tiers, a handful of goods.
- **Needs**: players must consume (abstracted food/housing/etc.) on a cadence. Needs create demand, stakes, and the possibility of deprivation — without which no economic system means anything. Consequences of unmet needs must be motivating but not punishing enough to drive churn (GDD question).
- **Labor** as the primary player action: choosing (or being assigned) work, allocating effort, possibly light skill/specialization progression. Whether labor is a pure allocation decision or involves some active interaction is a key fun-vs-scope GDD question.

### 4.3 Emergence over scripting

Firms, communes, unions, black markets, hoarding, shirking, mutual aid — none of these should be scripted events. Provide the primitives (contracts, transfers, communication, collective structures where the system permits them) and let them emerge. Observed emergence *is* the experimental output.

## 5. Player Experience

**Core loop (one session, ~5–20 min):** check the state of your household and your society → work / fulfill obligations → get compensated per your system's rules → consume to meet needs → allocate surplus (save, spend, invest, contribute — as the system allows) → check dashboards, react to what others are doing.

**Longer arcs:** specialization and roles (worker, manager, planner, entrepreneur, official — availability varies by system), accumulating wealth or standing, participating in collective decisions where the system has them, and societal-scale narratives (booms, shortages, reform movements) that emerge from aggregate behavior.

**Roles worth highlighting:** some systems *require* players in special roles (e.g., a planned economy needs planners; a market economy enables founders/employers). Whether these are elected, appointed, applied-for, or rotated is a per-system design question with big experimental implications.

## 6. Multiplayer Structure

- **Societies (servers):** small enough for players to perceive each other's behavior, large enough for a real economy. Initial guess: viable from ~20 concurrent-ish citizens, comfortable at 100–500. Needs modeling.
- **Time:** the economy advances in ticks (hours? day-cycles?) so that drop-in/drop-out play is fair — you shouldn't lose by sleeping. Absent players might default to a standing plan ("keep working my job, auto-buy food") with degraded efficiency, so presence matters but absence isn't ruinous.
- **AI participants:** to keep young or quiet societies liquid, agent citizens can fill the population. Given the agent-friendly design philosophy of my other projects, this could go further — an explicit API so people can field their own agents — but that changes the experiment's meaning (are we studying humans or strategies?). Flagged as a major open question.
- **Society lifecycle:** persistent, but possibly organized into long epochs with archived histories, so failed societies can conclude and their stories be studied rather than lingering forever.

## 7. Measurement & Comparison

The experiment and education goals live here; done well, it also feeds the game goal (people love dashboards).

- **Within a society:** live public statistics appropriate to that system — production, prices (where they exist), fulfillment of needs, distribution of wealth/standing.
- **Across societies:** a public observatory comparing societal outcomes on common metrics: total output, median welfare, need-fulfillment rates, inequality (Gini), participation/retention, mobility. Framing matters — comparisons should invite interpretation, not declare winners.
- **Telemetry:** log economic actions at a granularity that supports later analysis (and potentially write-ups). Privacy posture: pseudonymous by design; be explicit with players that behavior is studied in aggregate.

## 8. Design Principles & Scope Guardrails

1. **UI-only, forever.** No world, no avatars, no real-time spatial anything. If a feature needs a game world, it's out.
2. **System-neutral mechanics.** Reviewed explicitly: would a committed advocate of each system consider its implementation fair?
3. **The interface embodies the system.** Since the game is pure UI, the UI is where the system is *felt*. A planned economy's home screen shows your quota, your allocation, the plan's progress; a market economy's shows prices, your balance, open contracts. Same engine underneath, different lived interface — this is the primary vehicle of the north star.
4. **Small commodity economy, deep behavioral space.** Complexity budget goes to player interaction, not content.
5. **Web-first, low friction.** Join a society and be working within two minutes.
6. **Legible state.** Every number a player sees should be explainable; the economy is deterministic given actions. (Also makes it agent-friendly and debuggable.)

**Non-goals (v1):** politics beyond economic governance, war/conflict systems, geography/trade-between-societies, monetization design, mobile-native apps, historical scenario re-creation.

## 9. Relationship to Prior Work

- **The Capitol:** shares the fascination with player-driven economies (currency design, quality cascades, protocol-first thinking) but deliberately sheds the MMO world. Lessons on economic balance and backend architecture (Rust/Postgres/Redis, protocol-first, CLI/agent access) likely carry over to the TDD.
- **Taskman:** precedent for pure-UI presentation and tight scope.

## 10. Open Questions (seed list for the GDD)

1. What exactly is the labor interaction — pure allocation, or a light active task? What prevents botting from trivializing it (or is botting embraced via the agent question)?
2. Definitional rigor: what precisely operationalizes "pure communism" and "pure capitalism"? Which 4–6 presets ship first?
3. What are the consequences of unmet needs (efficiency loss? status loss? nothing permanent?) — stakes vs. churn.
4. How do special roles (planner, employer, official) get filled, and how is power abuse handled — by design, or observed as data?
5. Population dynamics: minimum viable society size, AI-citizen policy, and whether player-run agents are allowed, sandboxed, or separate.
6. Tick length and session cadence: what rhythm makes both a daily 10-minute check-in and a deep evening session satisfying?
7. Epochs vs. true permanence: do societies end, and what does "ending" mean?
8. Communication: in-game channels vs. external (Discord)? Communication affordances heavily shape collective behavior — this is an experimental variable in itself.
9. Identity & alts: one citizen per person per society? Alts would contaminate both the game and the data.
10. Can players propose/vote parameter changes (endogenous reform), or are systems fixed at creation (clean comparison)? Perhaps: fixed for "canonical" societies, mutable for "community" societies.

## 11. Next Steps

1. **GDD session:** resolve open questions §10, define the commodity graph, the core loop in detail, and the first preset systems with full rule specifications.
2. **TDD session:** architecture (likely Rust backend + typed protocol + web frontend), tick engine, persistence, telemetry pipeline, agent API posture.
3. **Prototype target:** one society, one system (probably pure capitalism first — fewest special roles), 10–20 humans, one week of persistent play.
