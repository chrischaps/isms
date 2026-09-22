# Neutrality review 01 — the Commune's copy (S2.9)

**Date:** 2026-09-21 · **Inputs:** `presets/lexicon/*.json`, `presets/copy/commune/welcome.md`, `presets/copy/commune/chronicle.toml`, `presets/copy/freeport/chronicle.toml` (the admission-vote lines) · **Against:** `neutrality-00.md` and GDD Appendix B items 1, 4 and 7, which are the ones copy can fail.

`neutrality-00` reviewed the rules and found the five presets' differences to be the constitutions' own. This note reviews the first preset whose *words* are written: does the Commune's copy describe the system the way its adherents describe it, without irony, and would a player who read only the interface know what this society believes?

## Item 1 — would a committed adherent recognise it?

The adherent is a council communist: free producers, common ownership of what they make, contribution by ability and draw by need, no money, no boss, no state that can compel. Read as them:

- **The Welcome Brief** says the workplaces belong to everyone and the product goes to the Store; that the norm asks and nobody can make you; that the Ledger is public; that the rule for a short shelf is the assembly's; that the coordinators publish and open but cannot assign, alter a draw or punish. Every sentence is a claim the adherent makes for themself. The one they might resist is "nobody can fire you", because it names an employer's act in a place with no employer; it stays, because a newcomer from Freeport needs the negative stated.
- **The Chronicle's headlines** put the assembly as the subject of every change ("The assembly carries", "The assembly sets the norm", "The assembly recalls"), the coordinator as the subject of the three powers only, and the Store as the subject of provision and of shortfall ("The Store did not reach them", "The Store fell short"). That is the adherent's own account of where agency sits.
- **The lexicon** calls the pay record a Draw record, the job a Contribution, the score The Record, the coordinator's room Coordination. No word borrows an office's or a market's dignity.

## Item 4 — without irony

Three places were candidates for a wink and were read twice:

- `PlanPublished`: "It asks; it does not command." This is the GDD's own description of an advisory Plan (§6.2 "it's advisory"). A Directorate reader would find it pointed; a Commune reader finds it the point. It stays because it describes the rule exactly and a coordinator would say it.
- `PolicyChanged.monitoring=high`: "The Ledger will now say exactly what each of us made." The GDD says a commune that votes surveillance of its members "has told us something"; the headline must not say that for us. It reports what the vote does and nothing about what it means. Kept.
- `OfficeUnfilled`: "The Commune has no coordinators, or too few." The card's phrase, made true for the event (which fires when seats are short, not only when they are all empty). The second clause is accuracy, not a shrug.

Nothing in the copy calls the Store a queue, the norm a quota, or the assembly a meeting. The hardship lines blame the Store, never the hungry.

## Item 7 — could a player tell what this society believes?

From the nav alone: Hours, Common Store, Ledger of Contribution, Assembly, and no Market, no Balance, no Payslip. From the first night's Chronicle: a verdict on whether everyone ate, a bare shelf named as the Store's failure, and any change to the rules attributed to a vote. Yes.

## Where the copy could still tilt, for a later read

1. **Freeport's admission lines** are the only ballot Freeport has and they read "The members carry …". A Freeport partisan may want the firm, not "the members", as the subject; the org kinds that vote there are member-owned, so the word is right, but the Freeport pass (Phase 5's visual review) should hear it aloud.
2. **The Directorate, Republic and Commonwealth lexicons** are first drafts written in one sitting with no Welcome Brief or Chronicle beside them; their own review comes with their copy cards in Phases 3–4. The GDD §15 rows are verbatim; the rest (Bulletin, Commendations, Merits, Directive, Bill) are mine and provisional.
3. **`need_met` fires every night** at 100 % fulfilment. It is the Commune's own verdict on itself (GDD §6.2 "the commune succeeds together or not at all"), and a reader who tires of it is reading a society that is working. If it reads as boasting in a live society, the threshold in `chronicle.toml` is the copy's, not the engine's.
