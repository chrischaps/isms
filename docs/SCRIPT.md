# How householders and legacy firms behave

Householders are the AI citizens that fill a society up to its population floor. They follow this script exactly, and nothing else. They never hold office, never vote, never found an org, never lend. This page is generated from the engine's `householder` module; if the two ever differ, the engine is wrong.

## Every householder, every hour

1. **Keep a plan.** Keep at least 24 Food in the pantry. Buy Wares when Comfort falls below 60 and the balance is above twice a cycle's living cost (24 Food and 4 Wares at the last price, plus rent). Never let the balance fall below 10% of all wages earned so far. No standing orders; no vote.
2. **Work.** If unemployed, accept the open job offer with the highest hourly-equivalent pay (piece rates are valued at the workplace's base output rate). Once hired, work the contract's full hours at normal effort.
3. **Live somewhere.** If unhoused, take the cheapest open lease whose rent is at most a quarter of last cycle's wages (or of eight hours at the legacy wage before any wages).
4. **Sell surplus.** Anything in the pantry that is not Food or Wares is offered on the book at the last price.

## Every legacy firm, every hour (managed by a householder)

1. **Hire** at the median of the open hourly offers (or the legacy wage when there are none): one open offer per workplace with free places, full hours, no fixed term, one cycle's notice. Stop hiring while the firm holds more than two cycles of full production of its output.
2. **Buy inputs** for one cycle of production at the current headcount, bidding at the last price plus the legacy markup, within the treasury.
3. **Sell output** at cost-plus: labor per unit at the legacy wage, plus inputs at the last price, times the markup, never below the start price. The ask is refreshed when that price moves.
4. **Invest** in one Machine at the last price whenever the treasury exceeds three cycles of payroll, and install every Machine held.
5. **Let dwellings** it owns at the legacy rent, one open offer per empty dwelling.
6. **Stay for sale.** Every share the firm still holds is on offer at book value (treasury plus inventory and machines at last price, per share); the offer is relisted when book value moves by more than ten percent. Whoever buys more than half becomes the controlling owner and manager; the householder steps down.

## When humans arrive

Each cycle end, householders are kept at `floor − active humans`. The most recently arrived householders leave first: their orders and offers are cancelled, their jobs and tenancies end, any firm they managed passes to another householder, and their money and goods leave the economy.
