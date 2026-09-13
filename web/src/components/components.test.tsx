// S1.7 done gate: component tests for Num/Explain and Meter.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Countdown, formatUntil } from "./Countdown";
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

describe("Meter", () => {
  it("fills to the value and marks the hardship line", () => {
    render(<Meter label="Food" value={76.3} />);
    const meter = screen.getByRole("meter", { name: "Food" });
    expect(meter.getAttribute("aria-valuenow")).toBe("76.3");
    expect((screen.getByTestId("meter-fill") as HTMLElement).style.width).toBe("76.3%");
    expect(screen.getByText("76")).toBeTruthy();
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
