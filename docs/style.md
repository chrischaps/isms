# Isms Style Bible — "Companion"

**Status:** v0.1, Sept 2026 · **Owner:** Chris · **Applies to:** `web/` (React 19 + TypeScript + Tailwind) · **Companion files:** `tokens.css` (source of truth for values), `tailwind.preset.js` (utilities mapped to tokens), `reference.html` (rendered gallery of everything below).

**How to use this doc.** If you are building or changing a screen, read §1–§3 in full, then the component entries you touch in §7, then the screen's card in §10, then run the checklist in §12. Values live in `tokens.css`; this document explains what they mean and when to use them. A component that needs a value that isn't a token is a design gap — add the token, don't inline the number.

**Why this exists.** Isms must be legible to a high-school student or a curious adult with no economics background, without hiding a single fact from a player who wants to dig. The current build presents everything at one level. Companion keeps every fact and adds a hierarchy: the screen answers *am I okay and what should I do?* first, then shows the facts, then keeps the rules and raw figures one deliberate step away.

---

## 1. Principles

**Verdict before figures.** Every screen opens with a sentence a newcomer can read, written from the player's side of the screen. Numbers support the sentence; they don't replace it.

**Nothing is removed; everything has a level.** Every datum from the current build stays on its screen. What changes is which layer it lives in (§2). "Out of the way" means one tap away with a clear label, never buried or unlabeled.

**State reads at a glance.** Good / attention / critical are encoded in colour *and* form (a pill, a fill colour, a status line) so the screen can be skimmed in a second. Semantic colour is separate from the accent (§4.2).

**Explain everything, on demand.** Every number carries the engine's `Explain` payload; every unfamiliar term carries a `?`. Explanations are two sentences, in plain words, and never interrupt unless asked for.

**One shape, many societies.** The shell is neutral. Vocabulary comes from the lexicon and partisanship lives in the Chronicle, the Welcome Brief and empty-state copy — never in colour, layout or component choice (GDD §15). A widget's absence is a design statement; its style is not.

**Phone is the first screen.** Layouts are designed at 390px and grow, not designed at 1100px and squeezed. Check-in sessions (GDD §9.2) happen on phones.

---

## 2. The information hierarchy

Every screen sorts its content into five layers. When in doubt about where something goes, ask: *does a newcomer need it to decide what to do next?*

| Layer | What it holds | How it appears | Example (Home) |
|---|---|---|---|
| **1 · Verdict** | One sentence: your state and what matters most right now | `Verdict` component, `--text-lg`, top of the first card | "You're **well fed** and **working today**, but you **don't have a place to live** yet." |
| **2 · Glance** | The three-to-six facts that justify the verdict, each with a status line | `NeedCard`s, status `Pill`s, big figures | Food 100 — "Full. You eat 1 food every hour from your pantry." |
| **3 · Facts** | Everything else the current screen shows today, as label/value rows | `FactList`, `Ledger` rows, plain lists | Balance · Pantry · Dwelling · Work |
| **4 · Detail** | Rules, budgets, multipliers, history, raw tables | `More` disclosure ("More about today's work"), secondary screens, tabs | Hour budget 8 h; effort ×1.0; output ×0.70 |
| **5 · Explain** | Definitions and the engine's rule for a number | `Explain` (`?` → note), `Num` popover | "Food is your survival meter. It drops about 4 an hour…" |

Rules:

- A screen has **exactly one** Verdict. If you can't write it, the screen's purpose is unclear — fix that first.
- Layer 2 holds at most six items. More than that and it's layer 3.
- Layer 3 is always visible. It is the "no information removed" guarantee.
- Layer 4 is collapsed by default on phones and may be open by default on `md+` when space allows, but it is still labelled as detail (a `More` summary or a section heading with `--muted` treatment).
- Layer 5 is never visible until asked for, and closing it leaves no trace.
- **Density is not a layer.** Do not add a "show more/less" that hides layer 3. (Playbook's Glance/Coach/Everything switch was rejected with the direction; don't reintroduce it.)

---

## 3. Mobile first

**Breakpoints** (from `tailwind.preset.js`): `sm` 480px, `md` 720px, `lg` 960px. Write styles for the phone and add `md:` for side-by-side layouts. Nothing may set a `min-width` wider than 360px.

**Stacking rule.** Every grid stacks to one column below `md`. Card grids (`.two`, `.needs`) become a vertical stack with `--sp-3` gaps. Two-column pages (Market, Society) become a single column in reading order: verdict → glance → the primary list → the secondary list. Never side-scroll a page.

**Navigation.**
- `md+`: top bar (society, clock, account links) plus a horizontal `ScreenNav` row beneath it.
- `< md`: the top bar shrinks to society name + balance + countdown; `ScreenNav` becomes a fixed bottom `TabBar` with **five** slots: the four most-used screens for this society (Home, Work, Market, Society in Freeport — the capabilities hook decides) and a **More** slot that opens a sheet listing the rest (Standing plan, Organizations, Contracts, Talk, Archive, Profile, Societies, Operator). The current screen is highlighted with `--accent-soft` fill + `--accent` label, same as desktop.
- The TabBar pads its bottom by `env(safe-area-inset-bottom)`; page content pads its bottom by `--tabbar-h` + safe area so nothing hides behind it.

**Touch.** Every interactive element is at least `--touch` (44px) tall on `< md`, including `?` buttons (the visible glyph stays 18px; the hit area is padded). Primary actions on a screen ("Keep my plan", "Set my hours", "Place bid") sit at the bottom of their card, full-width on phones, so they fall in the thumb zone.

**Type never shrinks on phones.** Body stays `--text-md` (16px) — smaller text is what makes the current build feel dense. Page titles may drop from `--text-3xl` to `--text-2xl` below `sm`.

**Tables on phones.** A `Ledger`/table with ≤3 columns keeps its columns. With more, it becomes a card-per-row list: the first column is the row title, the remaining columns become label/value pairs. The order book and tape (Market) keep their columns inside their own `overflow-x:auto` container — the one permitted horizontal scroll, and only within that box.

**Sheets, not modals.** Anything that opens over the page on a phone (More menu, Explain for long notes, order form, adjust hours) is a bottom `Sheet` with a drag handle; on `md+` it may be a popover or inline.

**Sticky things.** The countdown ("next hour 7 s") stays visible on phones in the compact top bar, since it is the game's heartbeat. Nothing else is sticky.

---

## 4. Tokens

### 4.1 Colour

The palette is a cool, slightly minted neutral set with one teal accent and three semantic hues. All values are in `tokens.css`; both themes pass WCAG AA (≥4.5:1) for every text-on-surface pairing listed here, verified at authoring time.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | #F2F7F4 | #111917 | page ground |
| `--surface` | #FFFFFF | #1B2422 | cards, bars, inputs |
| `--surface-2` | #EAF1ED | #22302C | insets, stripes, disabled |
| `--ink` | #1B2726 | #E8F0ED | primary text |
| `--muted` | #5F706D | #9DAFA9 | labels, secondary text |
| `--faint` | #9AABA6 | #5F726C | placeholders, bullets — never for text that must be read |
| `--line` / `--line-strong` | #DCE6E1 / #C3D2CB | #2C3A36 / #3C4F49 | borders, tracks / hover & focus borders |
| `--accent` / `-hover` / `-soft` / `--on-accent` | #25705D / #1D5C4C / #DDF0E9 / #FFF | #5FC3A8 / #7AD3BB / #1E3B34 / #0F1F1B | interactive: links, primary buttons, selected nav, notes |
| `--good` / `-fill` / `-soft` | #28714F / #3E9A6C / #DCF2E4 | #62C48F / #4FB27C / #1C3A2B | fine / no action |
| `--attn` / `-fill` / `-soft` | #8E600E / #D9A23C / #FBEFD5 | #E8B34C / #D9A23C / #3A2E14 | act soon |
| `--crit` / `-fill` / `-soft` | #AE3F2A / #D9634C / #F8DFDA | #EF8672 / #D9634C / #3F221D | act now |
| `--info` / `-soft` | #2C6A8F / #DCEBF5 | #7FB8DC / #1B3241 | neutral notices |
| `--chart-1…5`, `--chart-grid`, `--chart-you` | see file | see file | series colours; the player's own series is always `--chart-you` (ink) |

The `-fill` variants exist because a bar needs more saturation than text does to read at 8px tall; the text variants are darker so they pass contrast on the `-soft` fills. Use text tokens for text and icons, fill tokens for bars and swatches, soft tokens for backgrounds behind text in the same family.

### 4.2 Semantic colour rules

- **Good, attention and critical mean states of the player's world.** They are never used decoratively, never for branding, never as an accent. The accent (teal) means *you can interact with this*.
- Assign status by engine thresholds, not by taste: a need meter is `good` ≥ 50, `attn` 20–49 or *falling with no plan rule that stops it* (e.g. Shelter with no dwelling), `crit` < 20 or in hardship. A missing dwelling is `attn`. Hardship, destitution, a missed payment, a rejected command are `crit`.
- Every status colour is paired with words. A red bar with no status line is a bug.
- At most one `crit` element should be competing for attention on a screen; if several apply, the Verdict names the worst and the rest are `attn`-toned in their own cards.

### 4.3 Type

Two faces. **Outfit** (700, 500) is the display face: page titles, card titles, big figures, verdict emphasis. **Atkinson Hyperlegible** (400, 700, italic) is the body face, chosen for its distinguishable letterforms at small sizes. Self-host both via `@fontsource/outfit` and `@fontsource/atkinson-hyperlegible` (do not load from Google Fonts at runtime — the game must work with no third-party requests).

Scale (`--text-*`): 13 · 14 · 16 · 19 · 22 · 28 · 32. Body is 16. Uppercase labels are 13px with `--tracking-caps`. Running text is capped at ~65ch. Every figure is `tabular-nums` (set on `body`).

### 4.4 Space, shape, elevation, motion

Spacing is a 4px scale (`--sp-1…12`). Card padding is `--sp-5` on phones and `--sp-6` on `md+`; gaps between cards `--sp-4`. Radii: `--r-sm` 8 (inputs, notes), `--r-md` 12 (need cards, buttons, rows), `--r-lg` 16 (cards), `--r-pill`. Shadows are for things that float (menus, sheets, toasts) — cards use a `--line` border and no shadow. Motion is `--dur-fast`/`--dur-base` with `--ease`; disclosures animate height, nothing else animates on its own; `prefers-reduced-motion` disables all.

### 4.5 Theme mechanics

Light tokens are defined on `:root`. Dark is applied by `@media (prefers-color-scheme: dark)` guarded with `:root:not([data-theme="light"])`, and again by `:root[data-theme="dark"]`, so an explicit user choice beats the OS in both directions. Components read tokens only; they never contain a colour literal and never branch on theme. Tailwind's `dark:` variant exists but should be unnecessary. The theme choice is stored per browser (`localStorage`, wrapped in try/catch) and applied before first paint by an inline script in `index.html`.

---

## 5. Layout

**Shell.** `TopBar` (society name + preset tag, clock cluster, account links) above `ScreenNav`; both on `--surface` with a `--line` bottom border. On `< md` the account links move into the More sheet and `ScreenNav` becomes the `TabBar` (§3).

**Page width.** Single-column screens (Home, Work, Standing plan, Profile, Onboarding, Talk) use `--page-max` 820px. Dense screens (Market, Organizations, Contracts, Society, Archive) use `--page-max-wide` 1080px and may run two columns on `md+`. Side gutter `--gutter` 16px at every width, set once on the page wrapper.

**Page header.** `h1` in display face; a `--muted` meta row to the right (countdowns, "18 in all") that wraps under the title on phones. Home is the exception: its `h1` is a greeting ("Good morning, chaps.") because the screen's real title is the Verdict.

**Card rhythm.** A page is a vertical stack of `Card`s with `--sp-4` gaps. The first card holds the Verdict and the Glance layer. Cards that pair naturally (While you were away / Payslip) sit in a `.two` grid on `md+` and stack below it. Never nest a card in a card — use a `NeedCard` (a lighter inner tile) or a `FactList` inside a card instead.

**Footer strip.** Society-wide figures ("41 citizens · 1 people · 0 without work · price index 1.00") sit in a `--muted`, `--text-xs` row at the bottom of every screen. They are layer 3 and never disappear.

---

## 6. Iconography

Line icons only, 20px on a 24-unit grid, 2px stroke, round caps and joins, drawn in `currentColor`. One icon per concept, fixed across the app:

| Concept | Icon | Concept | Icon |
|---|---|---|---|
| Food | wheat stalk | Money / balance | coin (circle + cr glyph) |
| Shelter | house outline | Work | barn/factory outline |
| Comfort | four-point spark | Standing plan | list with a gear |
| Chronicle | page with lines | While you were away | clock |
| Warning (hardship) | triangle-bang | Market | two arrows crossing |
| Organizations | building | Contracts | handshake / signed page |
| Society | three people | Talk | speech bubble |

Icons accompany a label; they never replace one (the tab bar shows icon + label). No emoji anywhere in the shell. Illustration is welcome in onboarding and empty states only: flat, two-colour (`--accent` + `--muted`) spot illustrations at most 160px, never photographic.

---

## 7. Components

Names map to the TDD's shared component list where one exists (`Num`, `Meter`, `Ledger`, `OrderBook`, `TimeSeries`, `DiffSinceLastSeen`, `Countdown`). New names are introduced here. Each entry: purpose · anatomy · states · phone behaviour · rules.

### 7.1 `TopBar`
Society name (display, 20px) + preset tag (`--text-xs`, caps, `--surface-2` chip) · clock cluster: Balance (bold), Epoch · Day · time, tick progress bar (70×6, `--line` track, `--accent` fill), "next hour 4 s" · account links (Societies, Profile, Operator). Balance mounts only when `capabilities.money`.
*Phone:* society name, balance, countdown only; the rest in More.

### 7.2 `ScreenNav` / `TabBar`
Desktop: a row of text links, current one in `--accent-soft` pill with `--accent` bold text. Phone: fixed bottom bar, five slots (icon + 11px label), current slot tinted the same way; the fifth slot is More and opens a `Sheet`. Items come from `useCapabilities()` and labels from `useLexicon()`; the component never lists screens itself.

### 7.3 `PageHeader`
`h1` + meta row. Meta items are `--muted` with bold `--ink` values ("next hour **7 s**"). Wraps under the title below `sm`.

### 7.4 `Card`
`--surface`, 1px `--line`, `--r-lg`, padding per §4.4. Title `h2` (display 17–22px) with optional icon and `Explain`; optional `--muted` subtitle line. No shadow, no coloured border. A card's status is expressed by its content (pills, status lines), never by tinting the card.

### 7.5 `Verdict`
One sentence, `--text-lg`, at the top of the first card under the title "Right now". Emphasised phrases are bold and coloured by status (`--good`, `--attn`, `--crit`). Formula in §8.2. Screen-reader text is the plain sentence.

### 7.6 `NeedCard` (wraps `Meter`)
Inner tile (`--line` border, `--r-md`). Anatomy top-to-bottom: icon + name + `?` · value (display 28px) with "/ 100" in `--muted` 13px · 8px bar (`--line` track, status `-fill`) · status line (13.5px, `--muted`; `--attn`/`--crit` text when in that state) · hidden `Explain` note. Bar fill colour and status line follow §4.2 thresholds. Three `NeedCard`s sit in a row on `md+`, stack on phones.

### 7.7 `FactList`
Two-column `dl`: `--muted` label left, `--ink` value right-aligned, `--sp-2` row gap. Values may carry a `Pill`, a `--muted` gloss in parentheses ("22 food (about 22 hours)") and one inline action link ("adjust"). This is the layer-3 workhorse; every current label/value pair lives here.

### 7.8 `Pill`
Inline status chip: 12.5px bold, `-soft` background, matching text token, `--r-pill`. Variants: good, attn, crit, info, neutral (`--surface-2` + `--muted`). Used for states (none · open · in hardship · you work here), never for categories.

### 7.9 `Explain` (the `?`) and `Num`
`?` is an 18px circle button with `aria-expanded`; tapping toggles a `Note` directly beneath the element it belongs to (`--accent-soft` fill, `--r-sm`, 13.5px, ≤2 sentences). `Num` renders a value and, when the payload carries an `Explain`, the same `?` opens the rule: inputs as a `FactList` and the rule sentence beneath. On phones a note longer than three lines opens as a `Sheet`. Notes are written per §8.3 and pulled from the lexicon/copy files, not hard-coded in components.

### 7.10 `More` (disclosure)
`<details>` with a `--accent` bold summary, a small rotating triangle, and a dashed `--line` top border. Label names what's inside ("More about today's work", "Full order book"), never just "More". Closed by default on phones; may default open on `md+` if the card is otherwise short. Contains layer-4 content only.

### 7.11 Buttons
Primary: `--accent` fill, `--on-accent` text, `--r-md`, 10×16 padding, bold. Secondary: `--surface`, `--line` border. Danger: `--crit` fill for irreversible actions (found a firm, terminate a contract). Quiet: link-styled. One primary per card. Full-width below `sm`. Disabled: `--surface-2` fill, `--muted` text, and the reason stated beside it ("Costs 200.00 cr — you have 162.01").

### 7.12 `RuleList`
The Standing Plan rendered as a checklist: `--good` check icon + sentence with bold values ("Keep **Food** at least **24** in the pantry"). A rule that cannot currently execute (no money, no market) swaps the check for an `attn` icon and appends why. "Keep my plan" (primary) + "Edit plan" (secondary) beneath.

### 7.13 `Feed` (Chronicle, Diff)
List with a 6px dot per item. Hardship, default and rejection items are `hot`: `--ink` text, `--attn-fill` dot. Housekeeping items (epoch open/close, arrivals) are `--muted` with `--line` dots. Items link to the event. Same component renders `DiffSinceLastSeen` ("While you were away") with its "since Day 1, 5 AM" subtitle; empty state copy comes from the preset.

### 7.14 `Ledger` / tables
Header row: 13px caps `--muted`, `--line` rule beneath. Body rows: 1px `--line` separators, 44px min height on touch, figures right-aligned and tabular. Row hover `--surface-2`. Phone behaviour per §3. The player's own rows (your bid, your firm) are marked with a neutral `Pill` "you", never a colour.

### 7.15 `OrderBook` and tape
Bid side uses `--good-fill` depth bars, ask side `--crit-fill` at 25 % opacity behind figures — the one place red/green are used for a convention rather than a state, and they are labelled "bids"/"asks" in text. Your own orders get the "you" pill. On phones the book and tape are tabs inside the card, each with its own horizontal scroll container.

### 7.16 `TimeSeries` (uPlot)
Grid `--chart-grid`, axes text `--muted` 12px, series `--chart-1…5`, your series `--chart-you` 2px with an emphasised last point. A chart states its window in the card subtitle ("Last 3 days"). Flat lines are fine; never fake variance. Below `sm` the chart is 160px tall and the legend moves under it.

### 7.17 `Countdown`
"next hour **7 s**" / "payday **3 min**". Value bold `--ink`, label `--muted`. Ticks once a second without layout shift (fixed-width value).

### 7.18 Inputs
Text/number: `--surface`, `--line` border, `--r-sm`, 44px tall on touch, `--accent` focus ring. Select: same, with a chevron. Number inputs for hours/quantities show their unit as a suffix inside the field. Every field has a visible label; validation messages sit beneath in `--crit` and say what to change. The engine's rejection text is shown verbatim as the message.

### 7.19 Empty states
`--muted` sentence in the card body, from the preset's copy ("No payslip yet. The first comes at the end of the day."). Optionally one spot illustration in onboarding. Never a blank card.

### 7.20 `Sheet` and `Toast`
Sheet: bottom-anchored on phones (drag handle, `--surface`, `--r-lg` top corners, `--shadow-2`), centred dialog on `md+`. Toast: bottom-centre above the TabBar, `--ink` fill with `--bg` text, 4 s, one at a time; a rejected command toasts in `--crit-fill` with the engine's reason.

---

## 8. Voice and copy

### 8.1 Two voices
The **shell voice** (labels, verdicts, explainers, buttons, validation) is neutral, second person, present tense, sentence case, no jargon without a `?`. The **society voice** (Chronicle headlines, Welcome Brief, empty states) is the preset's and may be partisan (GDD §15). Never let one leak into the other: an explainer never editorialises; a Chronicle line never explains a rule.

### 8.2 Writing a Verdict
Shape: *[good thing], and [good thing], but [the one thing to act on].* Or, when nothing needs action: *[good thing] and [good thing]. Nothing needs you right now.* Name the state in the player's words (fed, housed, working, paid), not the meter's (Food 100). Bold the state words and colour them by status. One "but" maximum — the Verdict names the worst problem; others appear in their cards.

### 8.3 Writing an Explain note
Two sentences, at most three: **what it is** in plain words, **what moves it** (with the real numbers), and, if there is one, **what to do**. Example: "Shelter only drops while you're unhoused. Rent or buy a dwelling on the Contracts screen and it climbs back. Being unhoused also cuts your work output by 30 %." Numbers come from `Params`, interpolated, never typed in.

### 8.4 Status lines (under a meter or figure)
One short sentence: what's happening and, if it's changing, the rate. "Full. You eat 1 food every hour from your pantry." "Falling 2 an hour — you have no dwelling." Present tense, no exclamation marks.

### 8.5 Labels and numbers
Labels come from `useLexicon()`; never hard-code "Payslip". Money is `967.25 cr` (two decimals, space, unit after). Quantities carry their good: `22 food`. Rates: `8.00 cr/h`. Multipliers: `×0.70`. Time: `Day 1, 5 AM`; durations `7 s`, `3 min`, `8 h`. Percentages have a space: `98 %`. Use a gloss in parentheses to translate a figure into lived terms once per screen: "22 food (about 22 hours)".

### 8.6 Buttons and actions
Verb first, outcome named: "Keep my plan", "Set my hours", "Take this job", "Place bid". A confirmation names the consequence rather than asking "Are you sure?": "Found Iron & Sons for 200.00 cr — you'll have 762.01 cr after."

### 8.7 Errors
The engine's rejection text, verbatim, followed by what to change if the UI knows: "You asked for 9 hours; your budget today is 8. — Set 8 or fewer."

---

## 9. Accessibility

Contrast ≥ 4.5:1 for all text (checked for every token pairing in §4.1; `--faint` is decorative only). Visible focus ring on everything interactive. Touch targets ≥ 44px on `< md`. Status is never colour-only (§4.2). `Explain` uses `aria-expanded` and the note is in the DOM after its trigger. `Verdict` and status lines are real text, not icons with titles. Live figures (`Countdown`, balance) update inside `aria-live="off"` regions — announce ticks only on user action. Reduced motion disables all animation. Body text 16px minimum; the page must remain usable at 200 % zoom (the stacking rule handles it).

---

## 10. Screen guide

For each screen: Verdict (layer 1), Glance (2), Facts (3), Detail (4). Explain (5) applies to every number and term. Phone notes where the default stacking rule isn't enough.

**Home / Your Accounts** — Verdict: fed / housed / working / paid state. Glance: three `NeedCard`s. Facts: Balance, Pantry, Dwelling, Work; While you were away; Payslip; Standing plan as `RuleList`; Chronicle `Feed`; footer strip. Detail: hour budget, effort multipliers, output multiplier under "More about today's work". Phone: needs stack; "Keep my plan" full width.

**Work** — Verdict: "You're working 8 of 8 hours at Legacy Farm No. 1 at normal effort." Glance: hours used vs budget as a big figure with bar; effort as a segmented control with its cost line ("output ×1.0 · food decay ×1.0"). Facts: positions table (position, pay, hours, effort), payslip list, skill table. Detail: the two-workplace rule, fatigue, the full output formula under "How output is worked out". Phone: positions table becomes card-per-row; "Set my hours" full width.

**Standing plan** — Verdict: "Your plan keeps you fed and spends everything else." Glance: `RuleList` of consumption, saving, standing orders, vote default (when governance ≠ none). Facts: each rule's editable fields. Detail: last 24 executions of the plan (what it bought, when). Phone: one rule per card.

**Market** — Verdict: "Food costs 1.31 cr and hasn't moved in 3 days. Your plan will buy 2 food next hour." Glance: the selected instrument's last price, 3-day change as a `Pill`, spread; your open orders. Facts: instruments list, order book, tape, 3-day chart, "where it comes from". Detail: full share list with book values under "All shares", tape beyond 10 rows. Phone: instruments list becomes a horizontal chip row at the top; book/tape/chart become tabs in one card.

**Organizations** — Verdict: "18 firms are hiring; you work at Legacy Farm No. 1." Glance: your position; "Found one" as a primary action card with cost and balance-after. Facts: job board table; founding form. Detail: terms column (notice, open slots) collapses into each row's `More` on phones. Phone: "Take it" buttons are full-width in card-per-row.

**Contracts** — Verdict: "You have no dwelling and nothing to let is on the board." Glance: Shelter `NeedCard` alone; dwellings to let (when any). Facts: notice board by kind; your contracts with status. Detail: offer-a-loan and offer-goods forms in `More` sections; schedule previews. Phone: notice board card-per-row grouped by kind.

**Society** — Verdict: "Freeport is fed (98 %), one citizen is in hardship, prices are steady." Glance: need fulfillment, in hardship, price index, median wellbeing as four big figures with glosses. Facts: the remaining figures; Chronicle with day navigation; Standing table. Detail: consumption Gini and its definition; full citizens list. Phone: figures 2-up; Standing table card-per-row, your row pinned first.

**Talk** — no Verdict (it's a chat). Channel list as tabs; messages as a `Feed` variant with handles bold. Phone: channel picker is a `Sheet`.

**Archive** — Verdict: "Epoch 1 closed after 41 cycles." Glance: closing statement figures. Facts: per-epoch tables. Detail: exports.

**Profile / Societies / Operator** — plain `FactList`s and forms; API keys in a `More` with the agent-use notice. No Verdict needed.

**Onboarding** — the only place for illustration. Each step is one card, one primary button; the Welcome Brief is the society voice, set in body face at `--text-lg`. Target: working within two minutes (TDD S1.8).

---

## 11. Do / Don't

Do open with the Verdict; don't open with a table. Do keep every current figure on the screen; don't hide layer 3 behind a toggle. Do colour by engine thresholds; don't colour by taste. Do pair every colour with words; don't ship a bar without a status line. Do label disclosures with their contents; don't write "More" alone. Do use tokens; don't type a hex. Do stack to one column; don't scroll a page sideways. Do write two-sentence explainers; don't paste the GDD. Do keep the shell neutral; don't put partisanship in colour or layout. Do use the lexicon; don't hard-code "Payslip".

---

## 12. New-screen checklist

1. Write the Verdict sentence first. If you can't, stop.
2. List every datum the screen shows today; assign each a layer (2–4). None may be dropped.
3. Sketch the phone layout at 390px; then add `md:` for two columns if the screen is dense.
4. Build from the component catalog; add a component here before inventing one in a screen.
5. Every number → `Num` with `Explain` when the payload has one; every term a newcomer won't know → `?`.
6. Status colours only via §4.2 thresholds, each paired with a status line.
7. Labels via `useLexicon()`; widgets mounted via `useCapabilities()`.
8. Run both themes; run at 390px, 720px, 1100px; tab through with a keyboard.
9. Add the screen to `reference.html`'s screen gallery and to §10 above.

---

## Appendix — file map

```
web/
  src/styles/tokens.css          # this bible's values, in Tailwind v4 form (see below)
  src/styles/components.css      # the few pseudo-element rules utilities cannot say
  scripts/style-guard.mjs        # fails `pnpm check` on a hex, rgb(), `dark:` or a legacy class name
  src/components/                # §7 catalog: TopBar, ScreenNav, TabBar, Card, PageHeader, Verdict, NeedCard,
                                 #   FactList, Pill, Explain, More, Button, Field, RuleList, Feed, Ledger,
                                 #   OrderBook, TimeSeries, Countdown, Sheet, Toast, Icon, ThemeControl
  src/screens/Gallery.tsx        # the Storybook-free gallery route (TDD S1.7): the catalog at 390/720/1100, both themes
  src/screens/gallery/catalog.tsx
docs/style.md                    # this document
docs/style/reference.html        # rendered reference (static, from the direction review)
docs/style/tokens.css            # the values as plain CSS, the source for web/src/styles/tokens.css
docs/style/tailwind.preset.js    # the utilities → tokens mapping as a Tailwind v3 preset, for reference only
```

**Tailwind v4 (ADR-0013).** `web/` is on Tailwind v4, which is configured in CSS, so `tailwind.preset.js` is not loaded. Its intent is reproduced in `web/src/styles/tokens.css`: theme-invariant values (fonts, type scale, spacing, radii, breakpoints, containers) sit in `@theme static` under Tailwind's names; the colour tokens sit on `:root` under the names in §4.1; an `@theme inline` block maps every `--color-*` utility onto `var(--token)` and removes the stock palette, so `bg-surface text-muted border-line` are right in both themes and `bg-slate-500` does not compile. The pre-Companion names (`text-bad`, `bg-paper-2`, …) are aliased in a fenced `LEGACY` block until the sweep removes them. Dark is selected by `[data-theme="dark"]` rather than `:root[data-theme="dark"]` so the Gallery can render a dark island inside a light page; the OS-preference rule is as §4.5 says.

Fonts: `pnpm add @fontsource/outfit @fontsource/atkinson-hyperlegible`; import weights 500/700 and 400/700 (+400 italic) in `main.tsx`.
