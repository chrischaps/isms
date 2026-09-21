// The role workspaces' pure parts (S2.6, S2.8): what the builder may move,
// which kinds the land offers, and which rows of the Plan editor publish.

import { describe, expect, it } from "vitest";
import { movableKinds, splitToFractions } from "./BallotBuilder";
import { kindsOfLand, targetsToPublish } from "./Coordinator";

describe("BallotBuilder", () => {
  it("moves every enabled kind but the ones stood for or voted inside an org", () => {
    expect(movableKinds(["policy_change", "resolution", "election", "recall", "honor", "disbursement", "admission"])).toEqual([
      "policy_change",
      "resolution",
      "recall",
      "honor",
    ]);
    expect(movableKinds(["policy_change", "resolution"], ["policy_change"])).toEqual(["policy_change"]);
  });
  it("turns percentages into the engine's fractions", () => {
    expect(splitToFractions({ wares: 50, machines: 30, dwellings: 20 })).toEqual({ wares: 0.5, machines: 0.3, dwellings: 0.2 });
  });
});

describe("Coordinator", () => {
  it("lists the land's kinds once each, the limited ones first", () => {
    const plan = {
      slots: [
        { id: 0, kind: "farm" as const, workplace: 3 },
        { id: 1, kind: "farm" as const, workplace: null },
        { id: 8, kind: "mine" as const, workplace: null },
      ],
      unlimited_kinds: ["foundry", "mill", "workshop", "machine_shop", "builder"] as const,
    };
    expect(kindsOfLand({ slots: plan.slots, unlimited_kinds: [...plan.unlimited_kinds] })).toEqual([
      "farm",
      "mine",
      "foundry",
      "mill",
      "workshop",
      "machine_shop",
      "builder",
    ]);
  });
  it("publishes the rows with a number in them and leaves the blank and the negative alone", () => {
    expect(targetsToPublish({ 3: "40", 4: "", 5: " 12.5 ", 6: "-1", 7: "abc" })).toEqual({ "3": 40, "5": 12.5 });
  });
});
