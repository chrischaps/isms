// SB.1 done gate: the Companion primitives keep their contracts — tones map
// to token classes, the `?` is a proper disclosure, a disabled button says
// why, a fact list is a real dl, a verdict reads plain.

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Button } from "./Button";
import { Explain } from "./Explain";
import { FactList } from "./FactList";
import { Feed, RuleList } from "./Feed";
import { Icon } from "./Icon";
import { More } from "./More";
import { NeedCard } from "./NeedCard";
import { Pill } from "./Pill";
import { Verdict, verdictText } from "./Verdict";

describe("Pill", () => {
  it("maps every tone to its soft fill and text token", () => {
    render(
      <>
        <Pill tone="good">fed</Pill>
        <Pill tone="attn">no dwelling</Pill>
        <Pill tone="crit">in hardship</Pill>
        <Pill tone="info">open</Pill>
        <Pill>you</Pill>
      </>,
    );
    expect(screen.getByText("fed").className).toContain("bg-good-soft text-good");
    expect(screen.getByText("no dwelling").className).toContain("bg-attn-soft text-attn");
    expect(screen.getByText("in hardship").className).toContain("bg-crit-soft text-crit");
    expect(screen.getByText("open").className).toContain("bg-info-soft text-info");
    expect(screen.getByText("you").className).toContain("bg-surface-2 text-muted");
  });
});

describe("Explain", () => {
  it("is a disclosure: aria-expanded, the note after the trigger, no trace when closed", () => {
    render(<Explain note="Shelter only drops while you're unhoused.">Shelter</Explain>);
    const why = screen.getByRole("button", { name: "Explain" });
    expect(why.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("note")).toBeNull();
    fireEvent.click(why);
    expect(why.getAttribute("aria-expanded")).toBe("true");
    const note = screen.getByRole("note");
    expect(note.id).toBe(why.getAttribute("aria-controls"));
    expect(why.compareDocumentPosition(note) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    fireEvent.click(why);
    expect(screen.queryByRole("note")).toBeNull();
  });
});

describe("More", () => {
  it("is a details element whose summary names its contents", () => {
    render(
      <More summary="More about today's work">
        <p>Hour budget 8 h</p>
      </More>,
    );
    const details = screen.getByText("More about today's work").closest("details")!;
    expect(details).toBeTruthy();
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("Hour budget 8 h");
  });
});

describe("FactList", () => {
  it("renders label/value pairs with a gloss and an action", () => {
    const { container } = render(
      <FactList
        items={[
          { label: "Balance", value: "967.25 cr" },
          { label: "Pantry", value: "22 food", gloss: "about 22 hours", action: <a href="#">adjust</a> },
        ]}
      />,
    );
    expect(container.querySelectorAll("dl > dt").length).toBe(2);
    expect(container.querySelectorAll("dl > dd").length).toBe(2);
    expect(screen.getByText("Pantry").nextElementSibling!.textContent).toBe("22 food (about 22 hours) · adjust");
  });
});

describe("Button", () => {
  it("states why it is disabled and links the reason for a screen reader", () => {
    render(
      <Button variant="primary" disabledReason="Costs 200.00 cr — you have 162.01.">
        Found Iron & Sons
      </Button>,
    );
    const b = screen.getByRole("button", { name: "Found Iron & Sons" }) as HTMLButtonElement;
    expect(b.disabled).toBe(true);
    expect(b.className).toContain("bg-accent");
    expect(document.getElementById(b.getAttribute("aria-describedby")!)!.textContent).toBe("Costs 200.00 cr — you have 162.01.");
  });

  it("is enabled and primary otherwise", () => {
    render(<Button variant="primary">Keep my plan</Button>);
    expect((screen.getByRole("button", { name: "Keep my plan" }) as HTMLButtonElement).disabled).toBe(false);
  });
});

describe("Verdict", () => {
  it("colours state words by tone and reads plain", () => {
    const parts = ["You're ", { text: "well fed", tone: "good" as const }, ", but ", { text: "unhoused", tone: "attn" as const }, "."];
    render(<Verdict parts={parts} />);
    expect(screen.getByTestId("verdict").textContent).toBe("You're well fed, but unhoused.");
    expect(screen.getByText("well fed").className).toContain("text-good");
    expect(screen.getByText("unhoused").className).toContain("text-attn");
    expect(verdictText(parts)).toBe("You're well fed, but unhoused.");
  });
});

describe("NeedCard", () => {
  it("keeps the meter contract and pairs the bar's colour with a status line", () => {
    render(<NeedCard label="Shelter" value={94} tone="attn" status="Falling 2 an hour — you have no dwelling." note="Rent a dwelling." />);
    expect(screen.getByRole("meter", { name: "Shelter" }).getAttribute("aria-valuenow")).toBe("94");
    expect(screen.getByText("Falling 2 an hour — you have no dwelling.").className).toContain("text-attn");
    fireEvent.click(screen.getByRole("button", { name: "Explain Shelter" }));
    expect(screen.getByRole("note").textContent).toBe("Rent a dwelling.");
  });

  it("goes crit under the hardship line by itself", () => {
    render(<NeedCard label="Food" value={12} status="Under the hardship line." />);
    expect(screen.getByTestId("meter-fill").className).toContain("bg-crit-fill");
    expect(screen.getByText("Under the hardship line.").className).toContain("text-crit");
  });
});

describe("Feed and RuleList", () => {
  it("marks hot items and shows the empty copy", () => {
    const { rerender } = render(<Feed items={[{ key: 1, hot: true, node: "1 went hungry." }, { key: 2, node: "Epoch 1 opens." }]} />);
    expect(screen.getByText("1 went hungry.").className).toContain("hot");
    expect(screen.getByText("Epoch 1 opens.").className).not.toContain("hot");
    rerender(<Feed items={[]} empty="Nothing happened to you while you were away." />);
    expect(screen.getByText("Nothing happened to you while you were away.")).toBeTruthy();
  });

  it("says why a rule cannot run", () => {
    render(<RuleList rules={[{ key: "rent", node: "Pay rent", blocked: "no dwelling to pay for" }]} />);
    expect(screen.getByText(/no dwelling to pay for/).className).toContain("text-attn");
  });
});

describe("Icon", () => {
  it("is decorative unless titled", () => {
    const { container, rerender } = render(<Icon name="food" />);
    expect(container.querySelector("svg")!.getAttribute("aria-hidden")).toBe("true");
    rerender(<Icon name="food" title="Food" />);
    expect(screen.getByRole("img", { name: "Food" })).toBeTruthy();
  });
});
