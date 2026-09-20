# Phase 1 playtest guide: a Freeport epoch, for one to three people

Written 2026-09-18 (S1.15). GDD §18 makes Phase 1's exit "retention and interviews"; ADR-0009 and ADR-0011 shrank the cohort to the people at hand. This guide is for that: one to three players, one Freeport epoch, one conversation each afterwards. It is also the record: fill in the parts marked **[fill in]** and the guide becomes the report.

**When this runs (ADR-0012, 2026-09-19).** Not now. Phase 1 closed on its automated gates, and every human playtest is Phase 5, after all five societies are built: the same one to three people live consecutive epochs in each preset on one build, and this guide is the Freeport section of the five-part guide written then. Nothing below is filled in until Phase 5; the `[H]` marks in `docs/playtest/phase<N>-visual.md` are read before it starts.

The rule from GDD §19, kept front and centre: ask **what did you feel**, never **was it fun**. "Fun" gets a verdict; a feeling gets a moment, and moments are what the next phase is designed from.

## 1. The epoch

- **Society.** A canonical Freeport on a fresh database (worlds seeded before 2026-09-18 do not load). Suggested: `isms-server seed --preset freeport --name freeport-1 --tick-seconds 3600`, so a day of the game is a day of the calendar and payday comes each evening. A faster clock (`--tick-seconds 600`, four game days per calendar day) is fine for a solo run when a week is too long; say which in §5.
- **Length.** The full 42 days is the design. If the epoch is cut short, end it from the Operator room rather than abandoning it: the archive, the closing statement and the rollover are part of what is being tested.
- **Who plays.** One to three humans with their own accounts; everyone else is a householder. Nobody plays with the Operator room open; the operator's end of the epoch is a separate role and should be someone else's if there is anyone else.
- **What they are told beforehand.** The Welcome Brief and nothing more. No hints about founding a firm, credit, or leases; whether people find those is part of the result.
- **A diary, two minutes a day.** Each player keeps a note per calendar day (a text file, a message to themselves), answering only: *what did I do, what surprised me, what did I not understand*. The interview leans on it; memory of day 3 is gone by day 30.

## 2. What to record without asking

Retention is not a criterion (ADR-0012: three people cannot measure it), but the days seen say when a person stopped, and that is a moment to ask about. From the server, per player, at the end (the numbers are all in the log; `docs/playtest/runs/` reports show how the harness pulls them):

| signal | where | why it matters |
|---|---|---|
| calendar days with at least one `CitizenSeen` | `events` by `citizen`, `received_at::date` | did they come back |
| days the standing plan was changed (`PlanChanged`) | `events` | did they steer or coast |
| orders, offers, contracts accepted (`OrderPlaced`, `*Offered`, `*Accepted`) | `events` | did they touch the economy |
| ticks in hardship, and when | `HardshipBegan` / `HardshipEnded` | the felt part has a timestamp |
| net worth and self-made on the final standing | `epoch_archives.summary.standings` | where they ended up |
| whether they left a closing statement | `epoch_archives.closing_statements` | did the ending mean something |

**[fill in]** one row per player:

| player | days seen of 42 | plan changes | market actions | hardship ticks | final net worth / self-made | closing statement |
|---|---|---|---|---|---|---|
| | | | | | | |

## 3. The interview

One sitting, 30 to 45 minutes, within two days of the epoch's end, with the player's diary and their archive page open. Record it (with consent) or take notes; either way the fill-in below is the deliverable. Ask in this order; follow the answer before moving on; never supply the word you are hoping for.

### 3.1 The first hour

1. Walk me through your first ten minutes. Where did you look first, and what did you do first?
2. What did the meters mean to you before anyone explained them?
3. Was there a moment early on when you thought "oh, I see"? What was it?

### 3.2 Money and work

4. Tell me about the first payday. What did you feel when the number arrived?
5. Did you ever change your hours or your effort? What made you?
6. Did you ever look at another employer's offer? What did you compare?
7. Was there a point where you stopped thinking about food? When?

### 3.3 Hardship, if it happened

8. Your meters show hardship on day N. What do you remember of that day? What did you try?
9. What did the game tell you about why? Was it right?
10. If you got out of it: what got you out? If not: what did you think would have?

### 3.4 Ownership, if it happened

11. You founded a firm / bought shares / let a dwelling. What did you want from it?
12. Did it feel like yours? What made it feel that way, or not?
13. What did you not understand about running it, and what did you do about that?

### 3.5 Other people

14. Who did you notice? Householders, the other players, a name in the Chronicle?
15. Did you talk to anyone? About what? If not, what would have made you?
16. Did you feel anyone was doing better than you? How did you know?

### 3.6 The society

17. Did you read the Chronicle? Which headline stayed with you?
18. Did you look at the numbers page or the scoreboard? What were you looking for?
19. If someone asked you what kind of place Freeport is, what would you say?

### 3.7 The ending

20. How did you learn the epoch was ending? What did you do with the last two days?
21. Read me your closing statement, or tell me why you did not leave one.
22. Looking at the archive: is that a fair picture of what happened? What is missing from it?

### 3.8 What was not there

23. Was there something you wanted to do and could not find, or found and could not do? (Note each one; this is the defects list's other half.)
24. What did you never understand? Say it in your own words.
25. If the same people played again next week in a society where everything is held in common and decided together, what do you think you would feel differently? (Do not describe the Commune; take whatever they imagine.)

### 3.9 Last

26. Is there a moment from the epoch you would tell someone about?

## 4. Writing it up

For each player, **[fill in]** one paragraph per section 3.1 to 3.9, in their words where possible, plus these three lines:

- **The moment.** The one thing they would tell someone (Q26), verbatim.
- **The gap.** The largest thing they did not understand or could not do (Q23, Q24), and whether it is a defect, a copy problem, or a design question.
- **The feeling that carried.** The emotion that came up most across the interview, named by them not by you.

Then, across players:

- **Days seen.** Days seen out of the epoch, and the day each person stopped if they stopped (a moment to ask about, not a criterion).
- **Defects filed.** New rows in `docs/playtest/phase1-defects.md`, one per gap, with a severity.
- **What v2 must answer.** Three to five design questions the interviews raised. Q25's answers go here verbatim; they are the Freeport baseline for the "felt difference" comparison across the other four sections of the Phase 5 guide.
- **The comparison to the synthetic cohort.** What the humans did that no persona did, and the reverse (the personas are in `agents/personas/`; their arcs are in `docs/playtest/runs/`).

## 5. The run itself

**[fill in]**

- Society id, seed, `tick_seconds`, start and end dates, how the epoch ended (scheduled, operator) and why if by hand.
- Build: the commit on main the server ran from.
- Who played, and who operated.
- Anything that broke during the run and what was done about it, with the defect ids.

## 6. Done means

For the Freeport section of Phase 5 (TDD §18.5, ADR-0012): §2's table has a row per player, §3 was asked of each player and §4 is written for each, §5 says which run this was. A guide with the questions but no answers is not done.
