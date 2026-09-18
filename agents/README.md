# Synthetic players (S1.16)

A cohort of players that play a `lab` society of Isms through the public API the way strategic people would, and write down where the game refused or confused them (ADR-0009, ADR-0010). Two kinds of brain drive them through the same tools: **scripted** strategies (free, deterministic, run in CI) and **LLM** personas on the Anthropic SDK (read the game's words, try what nobody scripted, report what they did not understand).

Nothing here reaches past the public API: no engine hooks, no database. The one exception is bootstrap, which mints each synthetic account's browser session with `isms-server session`, joins the society and creates the player's API key, exactly as a person would through the site.

## Running

```
make e2e-agents                                  # scripted cohort, two fast days, the CI gate; costs nothing
make agents                                      # mixed cohort, 8 players, 7 days at 10 s an hour (~30 min); needs ANTHROPIC_API_KEY
make agents BRAIN=llm PLAYERS=4 EPOCH_CYCLES=3   # every player on a model
make agents BRAIN=mixed TICK_SECONDS=5 RUN=friday
```

`make agents` seeds a fresh lab Freeport in its own database, serves it on a side port (`AGENTS_PORT`, default 18090), runs the cohort to the epoch's end or the budget's, writes `docs/playtest/runs/<RUN>.md`, and replays the log with `isms-server rebuild` as the conservation check. The run's own files are under `agents/runs/<RUN>/` (git-ignored): `accounts.json` (handles and keys, so a run can resume), one `<player>.journal.jsonl` per player, `<player>.notes.md` for LLM players, `stats.json`, `summary.json`, `report.md`.

Against a server you are already running: set `ISMS_URL`, `ISMS_SOCIETY` (a lab society's id), `DATABASE_URL` (for `isms-server session`) and run `pnpm play --run <name> [--players N] [--brain scripted|llm|mixed] [--model id] [--cycle-model id] [--max-usd n]`.

`pnpm report <run>` folds a run into the report again.

## Configuration

`config.toml` holds the models, the per-turn action cap, the concurrency, the budget and the persona list; CLI flags override it; a persona's frontmatter may override the models for itself. Models are chosen from a small table in `src/config.ts` that carries each one's price and thinking parameters (`claude-haiku-4-5`, `claude-sonnet-5`, `claude-opus-5`). The budget is enforced per run in tokens and dollars; a run that reaches it stops cleanly and still writes its report.

`models.cycle_provider` chooses where the once-a-day reflection runs: `api` (the SDK, on API credit) or `claude_code`, which hands that one call to the Claude Code CLI in headless mode (`claude -p --output-format json --json-schema ...`, no tools, no session) so it runs on the account's Claude subscription. The CLI must be installed and signed in; its tokens are counted in the budget under `<model> via claude-code` at no dollar cost. The hourly turns always use the SDK, which is where the tool runner is. `--cycle-provider` overrides it per run.

## Personas

`personas/*.md`: frontmatter (`name`, `slug`, `brain`, `goals`, `temperament`, `risk`, optional `model`, optional `max_actions_per_turn`) and a body the LLM brain reads as "who you are". They state goals, never a script. The scripted brain's strategies live in `src/brain/scripted/strategies.ts`, keyed by slug, with a householder-like floor for slugs without one; the rule-prober is the fuzzer in `src/brain/scripted/fuzz.ts`.

## Journals and the report

One JSONL line per turn: the clock, a small extract of the situation, the intent, every tool call with its result, every refusal verbatim (`code`, `detail`), what the player did not understand, how the turn ended, and the tokens it cost. Reflections (LLM players, once a day) and skipped hours share the file.

The report groups refusals by code and then by text with ids blanked, lists the fuzzer's findings (`DEFECT:` an accepted probe or a server error; `COPY:` a refusal that names things by raw id), groups confusions by the part of the game they name, draws one paragraph per player from the journal, and notes the turns the harness itself got wrong. Findings go to `docs/playtest/phase1-defects.md`; a known defect's probe name goes in `known-defects.txt` so `make e2e-agents` stays green until it is fixed.

## Tests

`pnpm check` (typecheck, oxlint, vitest) needs no server and no key. The tool layer is tested against recorded responses in `test/fixtures/api/`; `test/replay.test.ts` replays one player's recorded epoch from `test/fixtures/transcripts/` and asserts the journal matches line for line. To re-record after an API change: `AGENTS_CMD=record AGENTS_ARGS="--record rule-prober-1" PLAYERS=8 EPOCH_CYCLES=1 TICK_SECONDS=1 BRAIN=scripted RUN=rec bash scripts/agents/run.sh`, then copy `agents/runs/rec/{rule-prober-1.recording,signals,rule-prober-1.journal}.jsonl` into the fixtures. With a key in the environment and `BRAIN=llm`, the same command records a model-driven player, model exchanges included.
