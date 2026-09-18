import { join } from "node:path";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { HomeView } from "../src/api/client.ts";
import { fixtureName } from "../src/api/transport.ts";
import { extract, situationText } from "../src/player/situation.ts";

const home = (): HomeView =>
  (JSON.parse(readFileSync(join(import.meta.dirname, "fixtures", "api", fixtureName("GET /s/1/home")), "utf8")) as { body: HomeView }).body;

describe("situationText", () => {
  it("names only the flags that are set (the slacker caught the header listing every flag)", () => {
    const h = home();
    const all = { ...h, citizen: { ...h.citizen, flags: { in_hardship: false, destitute: false, defaulted: false, options_narrowed: false } } } as unknown as HomeView;
    expect(situationText(all)).not.toMatch(/flags:/);
    const one = { ...h, citizen: { ...h.citizen, flags: { in_hardship: true, destitute: false } } } as unknown as HomeView;
    expect(situationText(one)).toMatch(/flags: in_hardship\./);
  });
  it("is a function of the view alone", () => {
    expect(situationText(home())).toBe(situationText(home()));
    expect(extract(home())).toMatchObject({ jobs: home().labor.employment.length, housed: home().household.dwelling != null });
  });
});
