import { describe, expect, it } from "vitest";
import type { Clock } from "../src/api/client.ts";
import { EdgeDetector, edgesBetween, type TickSignal } from "../src/stream/ticks.ts";

const clock = (o: Partial<Clock>): Clock => ({
  epoch: 1,
  cycle: 1,
  tick: 1,
  engine_tick: 0,
  ticks_per_cycle: 24,
  epoch_ended: false,
  ...o,
});

describe("edgesBetween", () => {
  it("sees nothing on the first reading", () => {
    expect(edgesBetween(null, clock({}))).toEqual([]);
  });
  it("sees a tick when the engine tick moves", () => {
    expect(edgesBetween(clock({ engine_tick: 3, tick: 4 }), clock({ engine_tick: 4, tick: 5 }))).toEqual(["tick"]);
  });
  it("sees nothing when the clock did not move", () => {
    expect(edgesBetween(clock({ engine_tick: 3 }), clock({ engine_tick: 3 }))).toEqual([]);
  });
  it("sees a cycle then a tick at a day boundary", () => {
    expect(edgesBetween(clock({ engine_tick: 23, tick: 24 }), clock({ engine_tick: 24, tick: 1, cycle: 2 }))).toEqual(["cycle", "tick"]);
  });
  it("reports only the epoch's end once it has ended, never an hour to play", () => {
    const before = clock({ engine_tick: 47, tick: 24, cycle: 2 });
    const ended = clock({ engine_tick: 48, tick: 1, cycle: 3, epoch_ended: true });
    expect(edgesBetween(before, ended)).toEqual(["epoch_end"]);
    expect(edgesBetween(ended, ended)).toEqual([]);
  });
});

describe("EdgeDetector", () => {
  it("emits each edge once whichever source saw it", () => {
    const out: TickSignal[] = [];
    const d = new EdgeDetector((s) => out.push(s));
    d.seen(clock({ engine_tick: 0 }), "poll");
    d.seen(clock({ engine_tick: 1, tick: 2 }), "ws");
    d.seen(clock({ engine_tick: 1, tick: 2 }), "poll");
    d.seen(clock({ engine_tick: 2, tick: 3 }), "poll");
    expect(out.map((s) => [s.kind, s.via, s.clock.engine_tick])).toEqual([
      ["tick", "ws", 1],
      ["tick", "poll", 2],
    ]);
  });
});
