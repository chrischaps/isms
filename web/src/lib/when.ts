// Event and deadline times in the header clock's units (S1.13e): a cycle is
// a day, and a tick is an hour when the day has 24 of them. Events carry a
// 0-based cycle and the engine's 0-based running tick; the clock view is
// 1-based. Everything a player reads goes through here, never "c3 t14".

export type Stamp = { cycle: number; tick: number };

/** "2 PM" for hour 14 of a 24-hour day (0 is midnight). */
export function hourName(hour: number): string {
  const h = ((Math.trunc(hour) % 24) + 24) % 24;
  const h12 = h % 12 === 0 ? 12 : h % 12;
  return `${h12} ${h < 12 ? "AM" : "PM"}`;
}

function hourOf(tick: number, perDay: number): string {
  const within = ((tick % perDay) + perDay) % perDay;
  return perDay === 24 ? hourName(within) : `hour ${within + 1} of ${perDay}`;
}

/** "2 PM" for the API's 1-based clock view. */
export function hourOfClock(clock: { tick: number; ticks_per_cycle: number }): string {
  return hourOf(Math.max(1, clock.tick) - 1, clock.ticks_per_cycle);
}

/** "Day 3" for an event's 0-based cycle. */
export function dayOf(cycle: number): string {
  return `Day ${cycle + 1}`;
}

/** "Day 3, 2 PM" for an event. */
export function whenOf(e: Stamp, perDay = 24): string {
  return `${dayOf(e.cycle)}, ${hourOf(e.tick, perDay)}`;
}

/** "Day 3, 2 PM" for a bare engine tick (an order's expiry, a contract's start). */
export function whenOfTick(tick: number, perDay = 24): string {
  return `${dayOf(Math.floor(tick / perDay))}, ${hourOf(tick, perDay)}`;
}

/** "the end of Day 3" for the last tick of a day, else "Day 3, 2 PM". */
export function deadlineOfTick(tick: number, perDay = 24): string {
  return (tick + 1) % perDay === 0 ? `the end of ${dayOf(Math.floor(tick / perDay))}` : whenOfTick(tick, perDay);
}

/** The epoch's end, once announced two days before it (S1.15): what the
 *  clock says between the announcement and the last day. `null` before the
 *  announcement and once the next epoch has started. */
export function epochEndingText(clock: { cycle: number; epoch_ending?: number | null }): string | null {
  const last = clock.epoch_ending;
  if (last == null) return null;
  const left = last - clock.cycle;
  if (left < 0) return null;
  if (left === 0) return "This is the last day of the epoch. The ledgers close tonight.";
  if (left === 1) return `The epoch ends after Day ${last}: today and tomorrow remain.`;
  return `The epoch ends after Day ${last}: ${left + 1} days remain, counting today.`;
}
