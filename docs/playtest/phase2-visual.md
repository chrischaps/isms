# Phase 2 visual pass — the Commune beside Freeport (S2.11)

Written 2026-09-22 at the Phase 2 exit (TDD §18.5; ADR-0012). The kit pass put every shared component through a society with no money; the captures under `target/review-s211/` (`REVIEW_OUT=../target/review-s211 CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-phase2.spec.ts`) show every screen a Commune mounts beside its Freeport sibling at 390 / 720 / 1100 in both themes, with the horizontal overflow at 390 printed for each (0 on every screen) and any "cr" on a Commune page named (one finding, fixed: the Contracts sale and wanted forms).

What a machine can check is checked. What follows is what only a person can judge, marked `[H]`, for Phase 5 to read first (GDD §18 row 5). Pass for each mark means: the screen reads as the Commune's own and not as Freeport with the prices filed off; nothing on it asks for money; the first card says something to you before it shows you a figure.

## Reviewed pairs

| screen | Commune | Freeport | what differs by design (GDD §15) |
|---|---|---|---|
| Home | `commune-home-*` | `freeport-home-*` | no balance in the header or the facts; Store tile, Ledger line and tonight's Ballot as facts (Q156); the countdown reads "share-out", not "payday"; the compensation card is the Draw record under the Store icon; the dwelling gloss says one is assigned tonight |
| Work / Hours | `commune-work-*` | `freeport-work-*` | the norm picker instead of the job board; "by need, from the Store" where the pay line was; the advisory target |
| Standing plan | `commune-plan-*` | `freeport-plan-*` | no balance floor, no price ceiling, no standing orders; "draw" for "buy"; the vote default present |
| Organizations | `commune-orgs-*` | `freeport-orgs-*` | the collective and associations; no book value column; founding "costs nothing in money" |
| Contracts | `commune-contracts-*` | `freeport-contracts-*` | no credit card; the sale offer priced in goods for goods; no wanted ad (Q158) |
| Society | `commune-society-*` | `freeport-society-*` | Store stock where the price index was; the Record scored by hours given, the norm met and honors; the Coordination tiles |
| Talk | `commune-talk-*` | `freeport-talk-*` | the assembly channels beside the Square |
| Archive | `commune-archives-*` | `freeport-archives-*` | standings as Citizen · Honors with no rank (Q157); Store stock in the summary |
| Common Store, Ledger, Assembly | `commune-store-*`, `commune-ledger-*`, `commune-assembly-*` | (`freeport-market-*` for the Store) | Commune only: shelves as figures with a bar of stock against what is asked, the draw record under "Drew"; "Your record, day by day" in hours; the quorum bar |
| Gallery | `gallery-commune-*` | the rest of `/gallery` | the kit's Commune section: TopBar without a balance, Num in units and hours, both Ledgers, both Meters, the Diff narrating a default ballot |

## `[H]` marks for Phase 5

- `[H]` **Home: does "You're well fed, housed and working today" followed by a Draw record and a Ledger line feel like a household in a commune, or like a bank statement with the money removed?** The verdict's words are the same in both societies by design; whether the *facts beneath it* carry the difference is the judgement.
- `[H]` **Home: "share-out 22 s".** The label replaces "payday". Does a first-time player know what is shared out, or does the Store tile's "your share tonight" (on the Store screen) need to be nearer?
- `[H]` **Home: "Ledger of Contribution — 0.5 of 6 hours today".** Hours accrue by the tick (Q142), so the figure is a fraction most of the day. Does a person read 0.5 as half an hour, or as an error?
- `[H]` **Home: the Ballot fact reads "nothing before the assembly" on a quiet day.** Is a fact that says nothing worth its row, or should it appear only when a proposal is open? GDD §15 puts tonight's ballots on the Commune's Home; the row is the design statement even when empty.
- `[H]` **Store: the shelf bars.** Stock against this hour's requests, full when it covers them (Q154). With nobody asking, a shelf with one unit shows a full bar. Does the bar mislead, or does the figure beside it carry the number well enough?
- `[H]` **Store: "+27 food" as a surplus share at the day's end.** The householders' surplus is large in a 41-citizen lab Commune with one human. Does it read as generosity, as a glitch, or as the system's nature?
- `[H]` **Society: "Firms 0" on the Commune's Society screen.** The collective and the associations are orgs, but "Firms" is Freeport's word. A lexicon key (`org`) would fix it; is the word noticed?
- `[H]` **Society: the Record scores "Hours given · Norm met · Honors".** Is this a scoreboard a Commune partisan would be proud of, and a Freeport partisan would find dull — the neutrality test in reverse (GDD §15 Tone)?
- `[H]` **Archive: standings as Citizen · Honors with no rank.** In a lab Commune every row reads "—". Does an unranked table of dashes say "this society keeps no such score" or "the page is broken"? Q157's engine change (contribution in `Standing`) would give it something to say.
- `[H]` **Contracts: the sale offer priced in goods for goods ("For 2 wares the lot · Paid in wares").** Is barter what a Commune citizen expects a "sale" to be, and does the two-field form read as one price?
- `[H]` **Chronicle: "closing at the end of day 0".** The Commune's chronicle copy prints the engine's 0-based cycle where the clock says Day 1 (`presets/copy/commune/chronicle.toml`, S2.9). A copy fix, one line; noted here so Phase 5 does not file it twice.
- `[H]` **Onboarding: the plan step with one row ("Keep Food at least 24 — in the pantry, drawn from the Store").** A one-row fact list before "Keep these and go home": too thin to be a step, or exactly the Commune's plan?
- `[H]` **The Diff: "Your plan voted abstain on proposal #3 by default".** Proposal ids are 0-based on the wire (S2.10). Does "#0" read as a number a person would say aloud? (The Assembly shows the same ids.)
- `[H]` **Dark theme on the Commune.** Every pair was captured in both themes; the tokens are shared. Is there any screen where the Commune's tone (the surplus share, the bare shelf in crit) reads differently in the dark?

## Not a finding

- Overflow at 390 is 0 on every screen of both societies. The Gallery page (`/gallery`, a catalog with a width chooser, not a screen) measures 150 px over at a 390 viewport in its default "Fit" width; the elements the spec names are the shell demo's TopBar, which the Freeport shell section has carried since SB.2, so the Commune section did not add it. Pick "Phone" there.
- No Commune page says "cr" after the Contracts fix; the review spec prints any that returns.
- The Playwright specs cover what a machine can: the Commune spec asserts the Ledger and Ballot facts, the shelf bar, the hours record and the absence of "payday" and of a header balance (`web/e2e/commune.spec.ts`).
