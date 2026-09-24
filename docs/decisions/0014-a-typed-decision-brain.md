# 0014 — A typed-decision brain beside the scripted and model brains

**Status:** accepted · **Date:** 2026-09-24 · **Session:** SJ.1

## Context

ADR-0010 split the synthetic cohort into scripted brains (free, deterministic, the CI gate) and model brains on the Anthropic SDK (read the game's words, try what nobody scripted, say what they did not understand). ADR-0011 ended the model-driven runs at Chris's budget: an eight-player epoch cost $12–28, a Sonnet turn could not finish inside a 10 s tick (the LLM players skipped 20–64 hours of a seven-day run), and no run reached `epoch_end`. Since then no model has played a society; the scripted cohort presses on the rules with fixed heuristics and never has to *decide* anything.

TypeSafe's Jev, released 2026-09, is a "System One" model: no text out. One `POST` takes a JSON `state` and a map of typed questions (`choice` among described options, `score` on a described scale, `noul` yes/no) and answers each with a probability, at $0.042 per million input tokens and free output, ~450 ms median. It is on OpenRouter's Decisions API (`/api/alpha/decisions`) without a waitlist. Independent evals say it cannot count or compare numbers, its calibration is loose (an ECE of ~0.1), it always picks something, accuracy falls as irrelevant state grows, and narrow decomposed questions beat one broad one.

The harness's action surface already fits it: 29 of 33 act tools are ids, enums and integers, and every quantity and price a persona needs is computed in `agents/src/brain/scripted/strategies.ts`.

## Decision

1. **A third `Brain`, `jev`** (`agents/src/brain/jev/`), beside `scripted` and `llm`. Code reads the game and lays out the hour as *slots*, each a `choice` question whose options carry a tool call with the numbers already worked out (`candidates.ts`: `work`, `job`, and one of `venture` / `market` / `property` by persona), each ending in a do-nothing option. One decision call per turn; code runs the picks hours-first under the action cap. Jev never sees an id, a quantity it would have to compute, or a price it would have to compare: the comparison is in the option's sentence.
2. **The persona is in the questions, never in the state.** `state.ts` sends needs, balance, hours, the society's numbers and the slots' facts, deterministically and without prose beyond three headline strings; `questions.ts` puts `personaText` and "Decide only this: …" in each question's `instructions`. TypeSafe documents that text in the state can steer an answer; a headline should not vote.
3. **Confidence floors are tunables** in `agents/config.toml` `[jev]`: a choice under `min_confidence` (0.35) is read as its slot's do-nothing option; an irreversible one (switch jobs, found a firm, buy a dwelling) also needs `irreversible_confidence` (0.5) and 0.4 of the probability mass. They are guesses to be tuned from the report.
4. **`--brain jev` puts the `[jev].personas` on Jev and scripts the rest**, so every run carries its own contrast. The default four are the founder, the wage-maximiser, the speculator and the landlord. Freeport only for now: the Commune's slots (ballots, offices, the Plan, the Store) are a follow-up card, and `brainFor` refuses a society with an assembly.
5. **What Jev cannot replace, it does not pretend to.** It has no `reflect`, and `did_not_understand` only ever names an answer to a slot or option that was never offered. The comprehension data ADR-0009 exists for stays the language model's. Jev supplies the other half of the cohort's purpose: persona-flavoured judgment under strategic pressure, at a price and speed that fit CI and a 10 s tick.
6. **The journal line grows a `decisions` list** (slot, option, confidence, do-nothing, acted) and the report a "Jev decisions" table (held turns, mean confidence, choices the floors overruled, flip-flops, refusals, cost and latency a turn). The brain is judged by these against the scripted players in the same run, not by anything it says.
7. **One live run per explicit approval, as ADR-0011 stands.** Chris approved up to $1 for SJ.1's two runs (two days, then seven if clean). The model's price is listed in `MODELS` so the `Budget` prices every call; a run stops at `--max-usd` like any other.

## Amended by SJ.2 (2026-09-24)

The first runs showed the lab put few real choices in front of the brain: `work` was re-asked every hour and answered "leave it", `market` appeared twice in seven days, `property` never, and every player, scripted ones too, went unhoused for a week at output ×0.7. SJ.2 keeps decisions 1–7 and adds: a slot is offered only when something gives a reason to choose (`work` asks the full menu while nothing is set and afterwards only a change with a reason); `housing` and `plan` slots for every persona, `credit` for the borrower, and an honest firm slot (`fund_firm` from the founder's own pocket, two wage tiers so firms compete for hands); a `standing` block from the scoreboard in the state, with rank, trend and gaps precomputed, and one standing sentence in the questions; an `ambition` line per persona in the frontmatter, where "be the richest" lives for the personas who would think it and is absent for those who would not, so the constitution and the persona shape motive, not a global objective; and all seven of Freeport's strategic personas on Jev, so their choices are each other's opportunities. The Commune's slots stay a follow-up.

## Consequences

- A model plays Freeport again for cents an epoch and inside the tick; the CI gate (`make e2e-agents`) is unchanged and still costs nothing.
- The state builder must stay a pure function of the view, or the replay test's canonical-body comparison fails on the Jev exchange; the recording holds it as target `"other"`.
- The `run.brain` enum, `Persona.brain`, the journal's `brain` and `MODELS` widen; `MODELS.thinking` is optional because a decision model thinks in neither of the Anthropic ways, and the turn and cycle models are checked to be language models.
- Tests: `agents/test/jev.test.ts` (laying out the hour, the state's determinism and its lack of ids and persona, answers to tool calls, the floors, the tier order, an option never offered, a failing API, a refused key, the report's section).
- `OPENROUTER_API_KEY` joins the root `.env`; `.env.example` documents it.
