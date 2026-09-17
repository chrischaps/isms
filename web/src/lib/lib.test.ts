// What a player reads in place of "c3 t62" and "org #15".

import { describe, expect, it } from "vitest";
import type { EventRef } from "../api/client";
import { buildNames, workplaceTitles } from "./names";
import { payslipRows } from "./payslips";
import { dayOf, deadlineOfTick, hourName, hourOfClock, whenOf, whenOfTick } from "./when";

describe("when", () => {
  it("names hours as the header clock does", () => {
    expect(hourName(0)).toBe("12 AM");
    expect(hourName(12)).toBe("12 PM");
    expect(hourName(14)).toBe("2 PM");
    expect(hourName(23)).toBe("11 PM");
  });

  it("reads an event's 0-based cycle and running tick as a day and an hour", () => {
    expect(dayOf(41)).toBe("Day 42");
    expect(whenOf({ cycle: 2, tick: 62 })).toBe("Day 3, 2 PM");
    expect(whenOfTick(62)).toBe("Day 3, 2 PM");
    expect(whenOfTick(0)).toBe("Day 1, 12 AM");
  });

  it("agrees with the 1-based clock view", () => {
    // Engine tick 62 is the clock's cycle 3, tick 15.
    expect(hourOfClock({ tick: 15, ticks_per_cycle: 24 })).toBe("2 PM");
  });

  it("calls the last hour of a day the end of that day", () => {
    expect(deadlineOfTick(47)).toBe("the end of Day 2");
    expect(deadlineOfTick(46)).toBe("Day 2, 10 PM");
  });

  it("falls back to counted hours when a day is not 24 ticks", () => {
    expect(whenOf({ cycle: 0, tick: 2 }, 12)).toBe("Day 1, hour 3 of 12");
  });
});

describe("names", () => {
  const orgs = [
    { id: 15, name: "Greenfield", workplaces: [{ id: 4, kind: "farm" }] },
    {
      id: 16,
      name: "Ironworks",
      workplaces: [
        { id: 9, kind: "machine_shop" },
        { id: 7, kind: "machine_shop" },
        { id: 8, kind: "mine" },
      ],
    },
  ];
  const citizens = [
    { id: 1, handle: "ada" },
    { id: 2, handle: "otto" },
  ];
  const names = buildNames(orgs, citizens, 1);

  it("names orgs, citizens and parties, and the viewer as you", () => {
    expect(names.org(15)).toBe("Greenfield");
    expect(names.citizen(2)).toBe("otto");
    expect(names.citizen(1)).toBe("you");
    expect(names.party({ org: 16 })).toBe("Ironworks");
    expect(names.party({ citizen: 1 })).toBe("you");
    expect(names.party("society")).toBe("the society");
  });

  it("names a workplace by kind and org, numbered only among several of a kind", () => {
    expect(names.workplace(4)).toBe("farm at Greenfield");
    expect(names.workplace(7)).toBe("machine shop 1 at Ironworks");
    expect(names.workplaceTitle(9)).toBe("machine shop 2");
    expect(workplaceTitles(orgs[1]!.workplaces).get(8)).toBe("mine");
  });

  it("spells out an id only when the directory has no entry", () => {
    expect(names.org(99)).toBe("organization no. 99");
    expect(names.citizen(99)).toBe("citizen no. 99");
    expect(names.org(99)).not.toContain("#");
  });
});

describe("payslipRows", () => {
  it("dates a payslip by its day and names the org", () => {
    const slip = { seq: 7, cycle: 41, tick: 1007, kind: "Paid", payload: { Paid: { org: 15, amount: 6800 } } } as unknown as EventRef;
    const names = buildNames([{ id: 15, name: "Greenfield", workplaces: [] }], []);
    expect(payslipRows([slip], names.org)).toEqual([
      { key: "7", when: "Day 42", what: "Payslip, Greenfield", cents: 6800, explain: null },
    ]);
  });
});
