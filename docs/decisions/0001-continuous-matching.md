# 0001 - Continuous order matching

**Status:** accepted . **Date:** 2026-09-12 . **Session:** S0.8

## Context
GDD Q1 and Q6 want markets to move visibly during a session and agents to gain from reactivity; a batch auction at the tick would make every trade resolve hourly (TDD D5, T1).

## Decision
Each instrument is a continuous double auction. An order matches on submission against the opposite side in price-time priority (best price first, then lowest order id), trades at the resting order's price, and rests with any remainder. Bids escrow `remaining x limit` in money and are refunded the price improvement per fill; asks escrow the goods. Ticks only expire orders (default: end of the next cycle) and record the tick's VWAP per instrument and the basket price index. Self-trades are allowed and visible in the log. Standing-plan orders are ordinary orders with `source = Standing`.

## Consequences
Trades happen between ticks and are ordinary events; the tick sees them through the book's per-tick accumulators. Revisit if playtests find sniping corrosive (T1).
