// S1.7 done gate: component tests for Num/Explain and Meter.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Countdown, formatUntil } from "./Countdown";
import { formatWorldTime, tickProgress } from "./WorldClock";
import { Ledger } from "./Ledger";
import { Meter } from "./Meter";
import { Num } from "./Num";

describe("Num", () => {
  it("renders the value and opens its Explain on demand", () => {
    render(
      <Num
        value="62.40"
        unit="cr"
        explain={{
          rule: "payroll_hourly",
          inputs: [
            ["tick_hours", 192],
            ["wage", 7.8],
          ],
          formula: "wage x tick_hours / 24",
          result: 62.4,
        }}
      />,
    );
    expect(screen.getByText("62.40")).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Explain" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog.textContent).toContain("payroll_hourly");
    expect(dialog.textContent).toContain("tick_hours");
    expect(dialog.textContent).toContain("192");
    expect(dialog.textContent).toContain("wage x tick_hours / 24");
    expect(dialog.textContent).toContain("= 62.40");
  });

  it("has no affordance without an Explain", () => {
    render(<Num value={3} />);
    expect(screen.queryByRole("button")).toBeNull();
  });
});

describe("Ledger", () => {
  const row = (key: string, epoch: number, when: string) => ({ key, epoch, when, what: "Payslip, Greenfield", cents: 6800 });

  it("draws a labelled line where a list crosses into an earlier epoch", () => {
    render(<Ledger rows={[row("3", 1, "Day 1"), row("2", 0, "Day 42"), row("1", 0, "Day 41")]} />);
    const lines = screen.getAllByTestId("epoch-divider").map((d) => d.textContent);
    expect(lines).toEqual(["Epoch 2 · this epoch", "Epoch 1 · ended"]);
  });

  it("draws none while every row is from one epoch", () => {
    render(<Ledger rows={[row("2", 1, "Day 2"), row("1", 1, "Day 1")]} />);
    expect(screen.queryByTestId("epoch-divider")).toBeNull();
  });
});

describe("Meter", () => {
  it("fills to the value and marks the hardship line", () => {
    render(<Meter label="Food" value={76.3} />);
    const meter = screen.getByRole("meter", { name: "Food" });
    expect(meter.getAttribute("aria-valuenow")).toBe("76.3");
    expect((screen.getByTestId("meter-fill") as HTMLElement).style.width).toBe("76.3%");
    expect(screen.getByText("76")).toBeTruthy();
  });

  it("explains itself from the label: described for a screen reader, pinned open by a tap", () => {
    render(<Meter label="Food" value={40} hint="Buy Food on the Market screen." />);
    const label = screen.getByRole("button", { name: "Food" });
    const tip = screen.getByRole("tooltip", { hidden: true });
    expect(tip.textContent).toBe("Buy Food on the Market screen.");
    expect(label.getAttribute("aria-describedby")).toBe(tip.id);
    expect(tip.className).toContain("hidden");
    fireEvent.click(label);
    expect(tip.className).not.toContain("hidden");
    // The meter keeps its own role and name beside the label.
    expect(screen.getByRole("meter", { name: "Food" }).getAttribute("aria-valuenow")).toBe("40");
  });

  it("clamps and turns bad below the threshold", () => {
    render(<Meter label="Shelter" value={12} />);
    expect(screen.getByTestId("meter-fill").className).toContain("bg-bad");
    render(<Meter label="Comfort" value={140} />);
    expect(screen.getByRole("meter", { name: "Comfort" }).getAttribute("aria-valuenow")).toBe("100");
  });
});

describe("Countdown", () => {
  it("formats the time until an instant", () => {
    const now = Date.parse("2026-09-13T12:00:00Z");
    expect(formatUntil("2026-09-13T12:00:41Z", now)).toBe("41 s");
    expect(formatUntil("2026-09-13T12:41:00Z", now)).toBe("41 min");
    expect(formatUntil("2026-09-13T19:41:00Z", now)).toBe("7 h 41 min");
    expect(formatUntil("2026-09-13T11:00:00Z", now)).toBe("now");
    expect(formatUntil(null, now)).toBe("—");
    render(<Countdown at="2099-01-01T00:00:00Z" label="Next tick" />);
    expect(screen.getByText("Next tick")).toBeTruthy();
  });
});

describe("WorldClock", () => {
  it("turns ticks into hours and the elapsed share into minutes", () => {
    const day = { epoch: 1, cycle: 6, tick: 15, ticks_per_cycle: 24 };
    expect(formatWorldTime(day, 0.5)).toBe("2:30 PM");
    expect(formatWorldTime({ ...day, tick: 1 }, 0)).toBe("12:00 AM");
    expect(formatWorldTime({ ...day, tick: 13 }, 0.999)).toBe("12:59 PM");
    expect(formatWorldTime({ ...day, tick: 24 }, null)).toBe("11:00 PM");
    expect(formatWorldTime({ ...day, ticks_per_cycle: 12, tick: 3 }, 0.5)).toBe("tick 3/12");
  });

  it("measures the tick's elapsed share from the next tick's due time", () => {
    const now = Date.parse("2026-01-01T00:00:00Z");
    const due = new Date(now + 2_500).toISOString();
    expect(tickProgress(due, 10, now)).toBe(0.75);
    expect(tickProgress(new Date(now - 1).toISOString(), 10, now)).toBe(1);
    expect(tickProgress(null, 10, now)).toBeNull();
    expect(tickProgress(due, 0, now)).toBeNull();
  });
});
