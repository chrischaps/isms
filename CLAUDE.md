# Isms — agent conventions

You are implementing the game described in docs/gdd.md according to docs/tdd.md.
Work one session card at a time (docs/tdd.md §18). Read: this file → your card → only the sections the card lists.
Provisional answers to spec gaps live in docs/QUESTIONS.md; deviations from the TDD in docs/decisions/. Read both before an engine card.

## Non-negotiables
- isms-core is pure: no I/O, no async, no clock, no unseeded randomness. State changes only via `apply(event)`.
- Every tunable constant lives in presets/*.toml. A literal in engine code that could be tuned is a bug.
- The constitution gates capability; `handle` rejects anything the society doesn't enable with `NotInThisSociety`.
- Events that give a citizen money or goods carry an `Explain`.
- Conservation of money and goods must hold after every event (run `conservation_check` in tests).
- Determinism: same state + same events ⇒ identical bytes. Never iterate a HashMap in the engine; use BTreeMap. Use `libm` for transcendental functions so Windows and Linux agree bit-for-bit.
- The web client is one client of the public API; no UI-only endpoints.
- Never invent mechanics. If the GDD/TDD is silent, write the question and your provisional answer in docs/QUESTIONS.md and proceed.

## Workflow
- Engine changes (crates/isms-core, crates/isms-sim, presets/) go through a PR: branch `s<phase>.<n>-<slug>`; one PR; squash merge; merge only on green CI. Linux CI is the only check of cross-platform determinism, so it must pass before the change reaches main.
- Everything else (server, store, web, docs, deploy) is committed straight to main, one commit per card or fix. Committing needs no check; pushing does: run `make push` in the background and start the next task. It checks HEAD in the `../Isms-check` worktree, running only what the diff can break (web-only: `web-check`; crates: fmt, clippy, test; docs: nothing), and pushes that sha on green. The pre-push hook (`make hooks`, once per clone) refuses any other sha on main.
- A red `make push` pushes nothing: fix it with a new commit before the next push. CI also runs on every push to main; a red main comes before anything else. Use a PR anyway when two agents are working at once or when you want review.
- `make check` must be green before you open a PR.
- Add tests named in the card's done gate. Engine sessions also keep the proptest suite green.
- Append your entry to docs/SESSIONS.md (template in tdd.md Appendix C). Record deviations as docs/decisions/NNNN-*.md.
- Do not start the next card if the current done gate is red; stop and report.

## Commands
- `make check` — fmt, clippy -D warnings, cargo test, pnpm typecheck/lint/test
- `make push` — check HEAD of main in ../Isms-check and push on green (`bash scripts/check-and-push.sh --no-push` to check only); log in .git/check.log
- `make dev` — Postgres in Docker + server (tick_seconds=10) + Vite (Phase 1)
- `make sim PRESET=freeport EPOCHS=5` / `make sim-check PRESET=…`
- `make api-types` — regenerate the TS client from /openapi.json (Phase 1)
- On Windows run make from Git Bash; GNU make is at `C:\Program Files (x86)\GnuWin32\bin`.

## Layout
crates/isms-core (engine) · isms-sim (headless) · isms-store (Postgres) · isms-api-types (wire) · isms-server (axum, actors, scheduler) · isms-cli · web/ (Vite React TS) · presets/ (constitutions, params, lexicon, copy) · deploy/
