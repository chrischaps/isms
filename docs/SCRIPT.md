# How householders and legacy firms behave

Householders are the AI citizens that fill a society up to its population floor. They follow this script exactly, and nothing else. They never hold office, never vote, never found an org, never lend. This page is generated from the engine's `householder` module; if the two ever differ, the engine is wrong.

## Every householder, every hour

1. **Keep a plan.** Keep at least 24 Food in the pantry. Buy Wares when Comfort falls below 60 and the balance is above twice a cycle's living cost (24 Food and 4 Wares at the last price, plus rent). Never let the balance fall below 10% of all wages earned so far, except to buy Food when the pantry is empty. No standing orders; no vote.
2. **Work.** If unemployed, accept the open job offer with the highest hourly-equivalent pay (piece rates are valued at the workplace's base output rate). Once hired, work the contract's full hours at normal effort.
3. **Live somewhere.** If unhoused, take the cheapest open lease whose rent is at most a quarter of last cycle's wages (or of eight hours at the legacy wage before any wages).
4. **Sell surplus.** Anything in the pantry that is not Food or Wares is offered on the book at the last price.

## Every legacy firm, every hour (managed by a householder)

1. **Hire** at a wage each workplace's board moves: one open offer per workplace with free places it can pay a cycle's wages for, full hours, no fixed term, one cycle's notice. Every cycle end the firm reads the board: if the firm's shelf of what the workplace makes grew (rule 3), the wage falls a step; otherwise, if an offer stood unfilled at the close, it rises a step; otherwise it stays. The wage never goes below the legacy wage nor above twice it. Stop hiring while the firm holds more than two cycles of full production of its output. An open offer the firm can no longer honour — no place it can pay for, a glutted shelf, or a wage the board has moved off — is withdrawn, and the offer is re-posted at the board's wage while there is a place to fill.
2. **Buy inputs** for one cycle of production at the current headcount, bidding at the last price plus the legacy markup, within the treasury.
3. **Sell output** at cost-plus: labor per unit at the legacy wage, plus inputs at the reference price, times a markup the shelf moves. Every cycle end the firm reads its shelf of each good it makes: if it holds more than it held at the last close, the markup falls a step; if the shelf is bare after a cycle of making, it rises a step; otherwise it stays. The markup never goes below zero (the firm sells at cost) nor above three times the legacy markup. The ask is refreshed when that price moves.
4. **Invest** in one Machine at the last price whenever the treasury exceeds three cycles of payroll, and install every Machine held.
5. **Let dwellings** it owns at the legacy rent, one open offer per empty dwelling.
6. **Stay for sale.** Every share the firm still holds is on offer at book value (treasury plus inventory and machines at last price, per share); the offer is relisted when book value moves by more than ten percent. Whoever buys more than half becomes the controlling owner and manager; the householder steps down.

## In the Republic

The same householder and the same legacy firms as in Freeport. The householder pays its income tax at every cycle end and takes the need floor when the treasury tops it up, like anyone else; it never changes its plan because of either, and a legacy firm never offers below the minimum wage (its offers start at the legacy wage, above it, and the board only moves them up from there). No householder ever forms a union, signs a collective agreement or strikes; those are for humans.

## In the Commonwealth

The same householder, in a society where every org is a cooperative and there are no wages:

1. **Keep a plan.** As in Freeport.
2. **Work.** With no position, ask to join the cooperative with a place still open that its steward would fill (a glutted coop admits nobody) whose last surplus per member was highest (the least staffed by the balancing weights, counting requests already waiting, breaks ties), one live request at a time. Once admitted, work the full eight hours at normal effort at the workplace the coop placed you. After two cycles as a member, if the coop's last share per member was under a cycle's living cost and another coop that is admitting shared more, leave (forfeiting this cycle's share) and ask there. A steward never leaves its coop.
3. **Live somewhere.** As in Freeport: dwellings are let by the Builders' coop.
4. **Sell surplus.** As in Freeport.

A legacy cooperative's householder manager runs the legacy firm's script with three differences: it admits members (pending requests, oldest first, while a workplace has room and the coop is not glutted) instead of posting job offers; when glutted (more than two cycles of full production unsold, or for Builders that many dwellings standing empty) it buys no inputs at all, so the loss a firm's treasury would carry falls on the members' share; its reserve before bidding for inputs is the obligations falling due (loan installments, the capital levy) plus a cycle of what its members would earn at the legacy wage; and it buys a Machine when the treasury, less what its bids this hour already commit, exceeds one and a half cycles of what its members would earn at the legacy wage plus the price; when it cannot afford one it applies to the Public Investment Bank for the price, once, and buys when the loan lands, and buys none past twelve per working member. Every cycle end the coop shares its surplus (this cycle's gain in the treasury, less capital received and obligations due) among its members by its rule: hours-weighted by default. There are no shares to sell.

## In the Commune

The same householder, in a society with no money, no prices and a Common Store:

1. **Keep a plan.** The plan says "follow the work norm"; the Food target and money rules do nothing here. Each hour the plan files a draw request for exactly the Food and Wares that would bring the meters to full (less what the pantry holds); the store serves everyone when it can and rations by the society's rule when it cannot.
2. **Work.** With no position, join the workplace with the fewest workers per unit of weight (the published `balance_weights`, favouring the Food chain). Once in a position, work the published norm (six hours by default) at normal effort. Hours and attributed output go on the public Ledger of Contribution each cycle.
3. **Live somewhere.** A dwelling from the collective stock is assigned on arrival and at every cycle end while one is free; nothing to rent.
4. **Sell nothing.** There is no market; whatever the store holds beyond everyone's needs is shared out equally at cycle end.

A collective's householder manager has one job: each hour, install one Machine from the Common Store at each of the collective's workplaces (never at the Machine Shop itself) while the store has any, so the society's Machines spread across its workplaces.

## In the Directorate

The same householder, in a society where the state owns every workplace, assigns labor and sells at a published price list:

1. **Keep a plan.** The plan says "accept my assignment". Keep at least 24 Food in the pantry and buy Wares when Comfort falls below 60 and the balance is above twice a cycle's living cost, both as purchase requests to the state store at the published price (there is nothing to bargain over). Never let the balance fall below 10% of all wages earned so far, except to buy Food when the pantry is empty.
2. **Work.** Work the full eight hours at normal effort at the workplace the Planning Committee assigned (on arrival, by the balancing rule). Never ask for a transfer.
3. **Live somewhere.** A state dwelling is assigned on arrival; nothing to rent.
4. **Sell nothing.** There is no market. The state pays each position at its wage grade every cycle end, with the plan bonus when the workplace met its target, and issues the minimum Food ration at zero price.

A state enterprise's householder manager installs one Machine from the state stock at each of its workplaces each hour while the stock has any, as in the Commune.

## The System's planner (simulation only)

Until the Planning Committee exists (Phase 4), the simulator plays the Committee with one rule: on the first hour of every cycle it publishes each producing workplace's target as last cycle's output times `sim_planner_target_growth` (1.05), and it approves every transfer request. It never moves prices, wage grades, the Materials split or ration cards.

## When humans arrive

Each cycle end, householders are kept at `floor − active humans`. The most recently arrived householders leave first: their orders and offers are cancelled, their jobs and tenancies end, any firm they managed passes to another householder, and their money and goods leave the economy (in a society with a Common Store or a state stock, their pantry and balance go back to it instead).
