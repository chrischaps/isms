// S2.11 done gate: the kit in a society with no money. Nothing here may say
// "cr" unless a row carries money; the Ledger takes goods and hours; the Diff
// narrates the assembly's news; the Figure carries a bar; the TopBar stands
// without a balance.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { EventRef } from "../api/client";
import { describe as describeEvent, DiffSinceLastSeen } from "./DiffSinceLastSeen";
import { Figure } from "./Figure";
import { Ledger } from "./Ledger";
import { Meter } from "./Meter";
import { Num, showValue } from "./Num";
import { StatTiles, storeStockFigure } from "./StatTiles";

const t = (k: string) => ({ compensation: "Draw record", job: "Contribution", proposal: "Proposal", society_stat: "Store stock" })[k] ?? k;

const drawExplain = {
  rule: "store_draw_by_need",
  inputs: [
    ["food_meter", 62],
    ["meter_per_unit", 18],
  ] as [string, unknown][],
  formula: "ceil((100 - food_meter) / meter_per_unit)",
  result: { Int: 2 },
};

describe("Num without a currency", () => {
  it("shows an Int or Float result with no unit, and money only where the value is money", () => {
    expect(showValue({ Int: 2 })).toBe("2");
    expect(showValue({ Float: 2.5 })).toBe("2.50");
    expect(showValue({ Money: 150 })).toBe("1.50 cr");
  });

  it("opens a goods Explain that never says cr", () => {
    render(<Num value="+2 food" explain={drawExplain} />);
    fireEvent.click(screen.getByRole("button", { name: "Explain" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog.textContent).toContain("store_draw_by_need");
    expect(dialog.textContent).toContain("= 2");
    expect(dialog.textContent).not.toContain("cr");
  });
});

describe("Ledger rows for draws and contributions", () => {
  it("renders a draw as goods under a named column, with no currency anywhere", () => {
    const { container } = render(
      <Ledger amountLabel="Drew" rows={[{ key: "1", epoch: 0, when: "Day 1, 6 AM", what: "Drew from the Store", goods: "+2 food", explain: drawExplain }]} />,
    );
    expect(screen.getByRole("columnheader", { name: "Drew" })).toBeTruthy();
    expect(screen.getByText("+2 food")).toBeTruthy();
    expect(container.textContent).not.toContain("cr");
  });

  it("renders hours given as +N h", () => {
    const { container } = render(
      <Ledger
        amountLabel="Hours"
        rows={[
          { key: "t", epoch: 0, when: "Day 2, so far", what: "Short of the norm of 6", hours: 4 },
          { key: "y", epoch: 0, when: "Day 1", what: "Met the norm of 6", hours: 6.5 },
        ]}
      />,
    );
    expect(screen.getByText("+4")).toBeTruthy();
    expect(screen.getByText("+6.5")).toBeTruthy();
    expect(screen.getAllByText("h")).toHaveLength(2);
    expect(container.textContent).not.toContain("cr");
  });

  it("still signs money where a row has it", () => {
    render(<Ledger rows={[{ key: "1", when: "Day 1", what: "Rent", cents: -800 }]} />);
    expect(screen.getByText("−8.00")).toBeTruthy();
    expect(screen.getByText("cr")).toBeTruthy();
  });
});

describe("DiffSinceLastSeen narrates the assembly's news", () => {
  const ev = (kind: string, payload: Record<string, unknown>, seq = 1): EventRef => ({ seq, tick: 23, cycle: 0, epoch: 0, kind, payload: { [kind]: payload } });

  it("says a ballot the plan cast by default", () => {
    const row = describeEvent(ev("Voted", { proposal: 3, citizen: 41, ballot: "abstain", by_default: true }), t, 41);
    expect(row.what).toBe("Your plan voted abstain on proposal #3 by default");
    expect(describeEvent(ev("Voted", { proposal: 3, citizen: 41, ballot: "yes", by_default: false }), t, 41).what).toBe("You voted yes on proposal #3");
  });

  it("says how a proposal closed, with the count", () => {
    const row = describeEvent(ev("ProposalClosed", { proposal: 3, passed: true, tally: { yes: 5, no: 2, abstain: 1, cast: 8, quorum: 4, eligible: 20 } }), t);
    expect(row.what).toBe("Proposal #3 carried (5 yes, 2 no, 8 of 20 cast)");
    expect(describeEvent(ev("ProposalClosed", { proposal: 4, passed: false, tally: { yes: 1, no: 6, abstain: 0, cast: 7, quorum: 4, eligible: 20 } }), t).what).toContain("failed");
  });

  it("says a seat taken, a seat emptied, an honor and an election", () => {
    expect(describeEvent(ev("OfficeTaken", { office: "coordinator", citizen: 41, term_ends_cycle: 6, approvals: 4 }), t).what).toBe("You took a seat as coordinator, through Day 7");
    expect(describeEvent(ev("OfficeVacated", { office: "coordinator", citizen: 41, reason: "term_ended" }), t).what).toBe("Your seat as coordinator emptied: the term ended");
    expect(describeEvent(ev("Honored", { citizen: 41, proposal: 4, cycle: 1 }), t).what).toBe("The assembly honored you (proposal #4)");
    expect(describeEvent(ev("ElectionOpened", { office: "coordinator", seats: 3, closes_cycle: 5 }), t).what).toBe(
      "An election opened for coordinator, 3 seats, closing at the end of Day 6",
    );
  });

  it("folds a disbursement like a transfer, in goods or in money", () => {
    const goods = describeEvent(ev("Disbursed", { proposal: 2, org: 7, to: { citizen: 41 }, asset: { good: ["food", 3] } }), t);
    expect(goods.goods).toBe("+3 food");
    expect(goods.cents).toBeUndefined();
    const money = describeEvent(ev("Disbursed", { proposal: 2, org: 7, to: { citizen: 41 }, asset: { money: 500 } }), t);
    expect(money.cents).toBe(500);
  });

  it("renders a Commune digest with a draw and a default ballot and no currency", () => {
    const { container } = render(
      <DiffSinceLastSeen
        t={t}
        me={41}
        events={[ev("Drew", { goods: { food: 2 }, explain: drawExplain }, 1), ev("Voted", { proposal: 3, citizen: 41, ballot: "abstain", by_default: true }, 2)]}
      />,
    );
    expect(screen.getByText("Drew from the Store")).toBeTruthy();
    expect(screen.getByText("+2 food")).toBeTruthy();
    expect(screen.getByText("Your plan voted abstain on proposal #3 by default")).toBeTruthy();
    expect(container.textContent).not.toContain("cr");
  });
});

describe("Meter as a shelf and a quorum bar", () => {
  it("takes a forced tone and a zero threshold for a bar that is not a need", () => {
    render(<Figure label="wares on the shelf" value="3" bar={<Meter label="wares against what is asked" value={38} threshold={0} tone="attn" bare />} tone="attn" status="8 asked, more than is here." />);
    const meter = screen.getByRole("meter", { name: "wares against what is asked" });
    expect(meter.getAttribute("aria-valuenow")).toBe("38");
    expect(screen.getByTestId("meter-fill").className).toContain("bg-attn-fill");
    expect(screen.getByText("8 asked, more than is here.").className).toContain("text-attn");
  });
});

describe("StatTiles without money", () => {
  const stats = {
    live: { population: 40, active_humans: 2, unemployed: 0, price_index: null },
    firm_count: 3,
    credit_outstanding: 0,
    last_cycle: { need_fulfillment_rate: 0.98, hardship_count: 0, median_wellbeing: 81, store_stock: { food: 31, wares: 12 } },
  };

  it("shows the Store stock where the price index would be, and no wage or credit", () => {
    const { container } = render(<StatTiles stats={stats} money={false} credit={false} orgs={true} t={t} />);
    const figure = screen.getByTestId("store-stock-figure");
    expect(figure.textContent).toContain("Store stock");
    expect(figure.textContent).toContain("31");
    expect(figure.textContent).toContain("12 Wares");
    expect(container.textContent).not.toContain("cr");
    expect(container.textContent).not.toContain("wage");
  });

  it("has no store figure before the first day closes", () => {
    expect(storeStockFigure({})).toBeNull();
  });
});
