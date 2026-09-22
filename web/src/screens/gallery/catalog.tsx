// The component catalog behind the Gallery (docs/style.md §7): every entry of
// the bible rendered with fixed data, so a visual check is one page in two
// themes at three widths. Keep the section titles: Playwright reads them.

import { useState, type ReactNode } from "react";
import type { EventRef } from "../../api/client";
import { Button, ButtonRow } from "../../components/Button";
import { Card, Stack, Tile, Two } from "../../components/Card";
import { Countdown } from "../../components/Countdown";
import { DiffSinceLastSeen } from "../../components/DiffSinceLastSeen";
import { Explain } from "../../components/Explain";
import { FactList } from "../../components/FactList";
import { Feed, RuleList } from "../../components/Feed";
import { Field, Input, Select } from "../../components/Field";
import { ICON_NAMES, Icon } from "../../components/Icon";
import { Ledger, TD, TD_NUM, TH, TH_NUM } from "../../components/Ledger";
import { Meter } from "../../components/Meter";
import { More } from "../../components/More";
import { ScreenNav, TabBar, TopBar, type NavItem } from "../../components/Nav";
import { NeedCard, Needs } from "../../components/NeedCard";
import { Num } from "../../components/Num";
import { OrderBook } from "../../components/OrderBook";
import { FooterStrip, PageHeader } from "../../components/PageHeader";
import { Pill } from "../../components/Pill";
import { Figure, Figures } from "../../components/Figure";
import { Segmented } from "../../components/Segmented";
import { Sheet, SheetRow } from "../../components/Sheet";
import { Tabs } from "../../components/Tabs";
import { TimeSeries } from "../../components/TimeSeries";
import { toast } from "../../components/Toast";
import { Verdict } from "../../components/Verdict";
import { WorldClock } from "../../components/WorldClock";

const explain = {
  rule: "payroll_hourly",
  inputs: [
    ["tick_hours", 192],
    ["wage", 7.8],
  ] as [string, unknown][],
  formula: "wage x tick_hours / 24",
  result: 62.4,
};

const ticks = Array.from({ length: 72 }, (_, i) => i);
const series = [
  { label: "Food", values: ticks.map((i) => 1.3 + 0.05 * Math.sin(i / 4)) },
  { label: "Wares", values: ticks.map((i) => 3.6 + 0.2 * Math.cos(i / 7)) },
  { label: "you", values: ticks.map((i) => 2.2 + 0.1 * Math.sin(i / 9)), you: true },
];

const events: EventRef[] = [
  { seq: 3448, tick: 23, cycle: 0, epoch: 0, kind: "Paid", payload: { Paid: { amount: 6240, explain } } },
  { seq: 3449, tick: 23, cycle: 0, epoch: 0, kind: "RentPaid", payload: { RentPaid: { amount: 800 } } },
  {
    seq: 3540,
    tick: 25,
    cycle: 1,
    epoch: 1,
    kind: "Trade",
    payload: { Trade: { instrument: { good: "food" }, buyer: { citizen: 41 }, qty: 2, price: 131 } },
  },
];

const TOKENS: { name: string; use: string }[] = [
  { name: "bg", use: "page ground" },
  { name: "surface", use: "cards, bars, inputs" },
  { name: "surface-2", use: "insets, stripes, disabled" },
  { name: "ink", use: "primary text" },
  { name: "muted", use: "labels, secondary text" },
  { name: "faint", use: "placeholders (decorative)" },
  { name: "line", use: "borders, tracks" },
  { name: "line-strong", use: "hover, focus borders" },
  { name: "accent", use: "interactive" },
  { name: "accent-soft", use: "selected nav, notes" },
  { name: "good", use: "fine" },
  { name: "good-fill", use: "bars" },
  { name: "good-soft", use: "pill fill" },
  { name: "attn", use: "act soon" },
  { name: "attn-fill", use: "bars" },
  { name: "attn-soft", use: "pill fill" },
  { name: "crit", use: "act now" },
  { name: "crit-fill", use: "bars" },
  { name: "crit-soft", use: "pill fill" },
  { name: "info", use: "notices" },
  { name: "info-soft", use: "notice fill" },
];

export type Section = { id: string; title: string; spec?: string; node: ReactNode };

// The shell's items with Freeport's words, as Society.tsx would build them.
const shellParams = { id: "1" };
const SHELL_NAV: NavItem[] = [
  { key: "home", to: "/s/$id", params: shellParams, label: "Your Accounts", short: "Home", icon: "home", exact: true, tab: true },
  { key: "work", to: "/s/$id/work", params: shellParams, label: "Work", icon: "work", tab: true },
  { key: "plan", to: "/s/$id/plan", params: shellParams, label: "Standing plan", icon: "plan" },
  { key: "market", to: "/s/$id/market", params: shellParams, label: "Market", icon: "market", tab: true },
  { key: "orgs", to: "/s/$id/orgs", params: shellParams, label: "Organizations", icon: "org" },
  { key: "contracts", to: "/s/$id/contracts", params: shellParams, label: "Contracts", icon: "contract" },
  { key: "society", to: "/s/$id/society", params: shellParams, label: "Society", icon: "people", tab: true },
  { key: "talk", to: "/s/$id/talk", params: shellParams, label: "Talk", icon: "talk" },
  { key: "archives", to: "/s/$id/archives", params: shellParams, label: "Archive", icon: "archive" },
];
const SHELL_ACCOUNT: NavItem[] = [
  { key: "societies", to: "/", label: "Societies", icon: "globe" },
  { key: "profile", to: "/profile", label: "Profile", icon: "person" },
  { key: "admin", to: "/admin", label: "Operator", icon: "gear" },
];
// The Commune's items (S2.11): the Store where the Market was, the Ledger and
// the Assembly present, nothing that needs money.
const COMMUNE_NAV: NavItem[] = [
  { key: "home", to: "/s/$id", params: shellParams, label: "Your Household & the Store", short: "Home", icon: "home", exact: true, tab: true },
  { key: "work", to: "/s/$id/work", params: shellParams, label: "Hours", icon: "work", tab: true },
  { key: "plan", to: "/s/$id/plan", params: shellParams, label: "Standing plan", icon: "plan" },
  { key: "store", to: "/s/$id/store", params: shellParams, label: "Common Store", icon: "store", tab: true },
  { key: "ledger", to: "/s/$id/ledger", params: shellParams, label: "Ledger of Contribution", icon: "ledger" },
  { key: "orgs", to: "/s/$id/orgs", params: shellParams, label: "Organizations", icon: "org" },
  { key: "contracts", to: "/s/$id/contracts", params: shellParams, label: "Contracts", icon: "contract" },
  { key: "assembly", to: "/s/$id/assembly", params: shellParams, label: "Assembly", icon: "assembly" },
  { key: "society", to: "/s/$id/society", params: shellParams, label: "Society", icon: "people", tab: true },
  { key: "talk", to: "/s/$id/talk", params: shellParams, label: "Talk", icon: "talk" },
  { key: "archives", to: "/s/$id/archives", params: shellParams, label: "Archive", icon: "archive" },
];
const drawExplain = {
  rule: "store_draw_by_need",
  inputs: [
    ["food_meter", 62],
    ["meter_per_unit", 18],
    ["shelf", 31],
  ] as [string, unknown][],
  formula: "ceil((100 - food_meter) / meter_per_unit), shelf permitting",
  result: { Int: 2 },
};
const normExplain = {
  rule: "contribution_hours",
  inputs: [
    ["tick_hours", 4],
    ["norm", 6],
  ] as [string, unknown][],
  formula: "sum of hours given today",
  result: { Int: 4 },
};
const communeEvents: EventRef[] = [
  { seq: 5102, tick: 23, cycle: 0, epoch: 0, kind: "Drew", payload: { Drew: { goods: { food: 2 }, explain: drawExplain } } },
  { seq: 5140, tick: 23, cycle: 0, epoch: 0, kind: "Voted", payload: { Voted: { proposal: 3, citizen: 41, ballot: "abstain", by_default: true } } },
  { seq: 5141, tick: 23, cycle: 0, epoch: 0, kind: "ProposalClosed", payload: { ProposalClosed: { proposal: 3, passed: true, tally: { yes: 5, no: 2, abstain: 1, cast: 8, quorum: 4, eligible: 20 } } } },
  { seq: 5160, tick: 23, cycle: 0, epoch: 0, kind: "ElectionOpened", payload: { ElectionOpened: { office: "coordinator", seats: 3, closes_cycle: 5 } } },
  { seq: 5200, tick: 30, cycle: 1, epoch: 0, kind: "OfficeTaken", payload: { OfficeTaken: { office: "coordinator", citizen: 41, term_ends_cycle: 6, approvals: 4 } } },
  { seq: 5201, tick: 30, cycle: 1, epoch: 0, kind: "Honored", payload: { Honored: { citizen: 41, proposal: 4, cycle: 1 } } },
];
const shellClock = { epoch: 2, cycle: 1, tick: 5, ticks_per_cycle: 24 };
const shellNext = new Date(Date.now() + 4_000).toISOString();

function Swatch({ name, use }: { name: string; use: string }) {
  return (
    <div className="border-line bg-surface overflow-hidden rounded-md border">
      <i className="block h-12" style={{ background: `var(--${name})` }} />
      <span className="block px-2.5 py-1.5 text-xs">
        <b className="text-ink block font-mono font-normal">--{name}</b>
        <span className="text-muted">{use}</span>
      </span>
    </div>
  );
}

function SegmentedDemo() {
  const [effort, setEffort] = useState("normal");
  return <Segmented label="Effort at Legacy Farm No. 1" value={effort} options={["low", "normal", "high"].map((v) => ({ value: v, label: v }))} onChange={setEffort} />;
}

function SheetDemo() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <ButtonRow className="mt-0">
        <Button onClick={() => setOpen(true)}>Open a sheet</Button>
        <Button variant="quiet" onClick={() => toast("Plan kept for Day 1.")}>
          Toast
        </Button>
        <Button variant="quiet" onClick={() => toast("You asked for 9 hours; your budget today is 8.", "crit")}>
          Toast, rejected
        </Button>
      </ButtonRow>
      <Sheet open={open} onClose={() => setOpen(false)} title="More" testId="gallery-sheet">
        {(["plan", "org", "contract", "talk", "archive", "person"] as const).map((n) => (
          <SheetRow key={n}>
            <Icon name={n} className="text-muted" />
            <span className="capitalize">{n}</span>
          </SheetRow>
        ))}
        <ButtonRow>
          <Button variant="primary" onClick={() => setOpen(false)}>
            Done
          </Button>
        </ButtonRow>
      </Sheet>
    </>
  );
}

export const SECTIONS: Section[] = [
  {
    id: "shell",
    title: "Shell: TopBar, ScreenNav, TabBar",
    spec: "§7.1, §7.2 — the society and its clock above a row of screens from 720px; below it the bar shrinks and the screens become the bottom TabBar with More. Items come in from the screen that mounts them.",
    node: (
      <div className="border-line -mx-4 overflow-hidden border-y">
        <TopBar
          name="Freeport"
          preset="freeport"
          balance={
            <span className="text-ink">
              <span className="text-muted hidden sm:inline">Balance </span>
              <b>967.25 cr</b>
            </span>
          }
          clock={<WorldClock clock={shellClock} nextTickAt={shellNext} tickSeconds={3600} />}
          phoneClock={<WorldClock clock={shellClock} nextTickAt={shellNext} tickSeconds={3600} phone />}
          account={SHELL_ACCOUNT}
        />
        <ScreenNav items={SHELL_NAV} />
        <div className="bg-bg text-muted px-4 py-6 text-sm">The page, under the bars.</div>
        <TabBar items={SHELL_NAV} account={SHELL_ACCOUNT} inline />
      </div>
    ),
  },
  {
    id: "num",
    title: "Num with Explain",
    spec: "§7.9 — a value with the engine's rule behind a `?`. Inputs as a fact list, the formula beneath.",
    node: (
      <p className="m-0">
        Last payslip: <Num value="62.40" unit="cr" explain={explain} />
      </p>
    ),
  },
  {
    id: "tokens",
    title: "Tokens",
    spec: "§4.1 — every colour on this page is one of these. Text tokens for text, -fill for bars, -soft behind text of the same family.",
    node: (
      <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
        {TOKENS.map((t) => (
          <Swatch key={t.name} {...t} />
        ))}
      </div>
    ),
  },
  {
    id: "type",
    title: "Type",
    spec: "§4.3 — Outfit for display, Atkinson Hyperlegible for body. Body is 16px and never smaller on phones.",
    node: (
      <div className="grid gap-3">
        {(
          [
            ["3xl · page title", "font-display text-3xl font-bold"],
            ["2xl · big figure", "font-display text-2xl font-bold"],
            ["xl · card title", "font-display text-xl font-bold"],
            ["lg · verdict", "text-lg"],
            ["md · body", "text-md"],
            ["sm · labels", "text-sm"],
            ["xs · caps", "text-xs font-bold tracking-caps uppercase text-muted"],
          ] as const
        ).map(([k, cls]) => (
          <div key={k} className="border-line grid grid-cols-[130px_1fr] items-baseline gap-4 border-b pb-2 max-sm:grid-cols-1">
            <small className="text-muted font-mono text-xs">{k}</small>
            <span className={cls}>You're well fed and working today.</span>
          </div>
        ))}
      </div>
    ),
  },
  {
    id: "icons",
    title: "Icons",
    spec: "§6 — line icons, 20px, 2px stroke, currentColor. One per concept; always beside a label.",
    node: (
      <div className="grid grid-cols-[repeat(auto-fill,minmax(110px,1fr))] gap-3">
        {ICON_NAMES.map((n) => (
          <div key={n} className="border-line bg-surface text-muted grid justify-items-center gap-1.5 rounded-md border px-2 py-3 text-xs">
            <Icon name={n} size={24} className="text-ink" />
            {n}
          </div>
        ))}
      </div>
    ),
  },
  {
    id: "card",
    title: "Card, Verdict, NeedCard, Figure",
    spec: "§7.4–7.6, §10 Society — the first card on Home: the Verdict, then three need tiles; and the Society's four glance figures with their status lines. Bars and status lines follow the engine's thresholds.",
    node: (
      <Stack>
        <PageHeader
          title="Good morning, chaps."
          meta={
            <>
              <Countdown at={new Date(Date.now() + 7_000).toISOString()} label="next hour" />
              <Countdown at={new Date(Date.now() + 180_000).toISOString()} label="payday" />
            </>
          }
        />
        <Card title="Right now">
          <Verdict
            parts={[
              "You're ",
              { text: "well fed", tone: "good" },
              " and ",
              { text: "working today", tone: "good" },
              ", but you ",
              { text: "don't have a place to live", tone: "attn" },
              " yet.",
            ]}
          />
          <Needs>
            <NeedCard label="Food" icon="food" value={100} status="Full. You eat 1 food every hour from your pantry." note="Food is your survival meter. It drops about 4 an hour and refills from your pantry; under 20 for a day is hardship." />
            <NeedCard label="Shelter" icon="home" value={94} tone="attn" status="Falling 2 an hour — you have no dwelling." note="Shelter only drops while you're unhoused. Rent or buy a dwelling on the Contracts screen and it climbs back." />
            <NeedCard label="Comfort" icon="spark" value={14} status="Low. Wares raise it; you have none in the pantry." note="Comfort is raised by using wares. It does not stop you working, but it counts toward wellbeing." />
          </Needs>
        </Card>
        <Card title="How Freeport is doing" icon="people">
          <Verdict parts={["Freeport is ", { text: "fed (98 %)", tone: "good" }, ", ", { text: "one citizen is in hardship", tone: "attn" }, ", ", { text: "prices are steady", tone: "ink" }, "."]} />
          <Figures>
            <Figure label="Need fulfillment" value="98" unit="%" status="Nearly every citizen-day above the line." />
            <Figure label="In hardship" value="1" tone="attn" status="At the end of yesterday." />
            <Figure label="Price index" value="1.00" status="Reference basket, Food = 1." />
            <Figure label="Median wellbeing" value="81" unit="/ 100" status="The middle citizen's needs, averaged." />
          </Figures>
        </Card>
      </Stack>
    ),
  },
  {
    id: "facts",
    title: "FactList, Pill, Explain, More",
    spec: "§7.7–7.10 — the layer-3 workhorse, status chips, the `?` on a term, and a labelled disclosure for layer 4.",
    node: (
      <Two>
        <Card title="Facts">
          <FactList
            items={[
              { label: "Balance", value: "967.25 cr" },
              { label: "Pantry", value: "22 food", gloss: "about 22 hours" },
              { label: "Dwelling", value: <Pill tone="attn">none</Pill> },
              { label: "Work", value: "Legacy Farm No. 1 · 8 h", action: <a href="#facts">adjust</a> },
            ]}
          />
          <More summary="More about today's work">
            <FactList items={[{ label: "Hour budget", value: "8 h" }, { label: "Effort", value: "×1.0" }, { label: "Output", value: "×0.70", gloss: "unhoused" }]} />
          </More>
        </Card>
        <Card title="Pills and Explain">
          <div className="flex flex-wrap items-center gap-2">
            <Pill tone="good">fed</Pill>
            <Pill tone="attn">no dwelling</Pill>
            <Pill tone="crit">in hardship</Pill>
            <Pill tone="info">open</Pill>
            <Pill>you</Pill>
          </div>
          <p className="mt-4 mb-0">
            <Explain note="Shelter only drops while you're unhoused. Rent or buy a dwelling on the Contracts screen and it climbs back. Being unhoused also cuts your work output by 30 %.">
              Shelter
            </Explain>{" "}
            is one of your three needs.
          </p>
        </Card>
      </Two>
    ),
  },
  {
    id: "actions",
    title: "Buttons and inputs",
    spec: "§7.11, §7.18 — verb first, one primary per card, full width below sm. A disabled button says why.",
    node: (
      <Card title="Set my hours">
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Hours today" unit="h" hint="Your budget today is 8.">
            <Input type="number" defaultValue={8} min={0} max={8} />
          </Field>
          <Field label="Effort" error="You asked for 9 hours; your budget today is 8. — Set 8 or fewer.">
            <Select defaultValue="normal">
              <option value="light">light</option>
              <option value="normal">normal</option>
              <option value="hard">hard</option>
            </Select>
          </Field>
        </div>
        <div className="mt-3 grid gap-1.5">
          <span className="text-sm font-bold">Effort</span>
          <SegmentedDemo />
          <span className="text-muted text-sm">output ×1.0 · Food decay ×1.0</span>
        </div>
        <ButtonRow>
          <Button variant="primary">Set my hours</Button>
          <Button>Edit plan</Button>
          <Button variant="danger">Terminate contract</Button>
          <Button variant="quiet">Cancel</Button>
        </ButtonRow>
        <ButtonRow>
          <Button variant="primary" disabledReason="Costs 200.00 cr — you have 162.01.">
            Found Iron &amp; Sons
          </Button>
        </ButtonRow>
      </Card>
    ),
  },
  {
    id: "lists",
    title: "RuleList, Feed, Ledger",
    spec: "§7.12–7.14 — the plan as a checklist, the Chronicle with hot items, and a table with caps headers and 44px rows.",
    node: (
      <Stack>
        <Two>
          <Card title="Your plan" icon="plan" subtitle="This runs every hour, even while you're gone.">
            <RuleList
              rules={[
                { key: "food", node: <>Keep <b>Food</b> at least <b>24</b> in the pantry</> },
                { key: "cash", node: <>Keep <b>0.00 cr</b> in hand; spend the rest</> },
                { key: "rent", node: <>Pay rent on <b>Dwelling No. 17</b> every day</>, blocked: "no dwelling to pay for" },
              ]}
            />
            <ButtonRow>
              <Button variant="primary">Keep my plan</Button>
              <Button>Edit plan</Button>
            </ButtonRow>
          </Card>
          <Card title="The Chronicle" icon="page" subtitle="What's happening in Freeport">
            <Feed
              items={[
                { key: 1, hot: true, node: "1 went hungry this cycle." },
                { key: 2, node: "Epoch 1 opens. Everything is for sale again." },
                { key: 3, node: "chaps arrives with 1000.00 credits and nothing else." },
              ]}
            />
          </Card>
        </Two>
        <Card title="Ledger">
          <Ledger
            rows={[
              { key: "1", epoch: 1, when: "Day 1, 11 PM", what: "Payslip, Iron & Sons", cents: 6240, explain },
              { key: "2", epoch: 1, when: "Day 1, 11 PM", what: "Rent, Dwelling #17", cents: -800 },
              { key: "3", epoch: 0, when: "Day 42, 3 AM", what: "Bought 2 food", goods: "2 food" },
            ]}
          />
          <More summary="Card-per-row on phones">
            <Tile>
              <div className="flex justify-between gap-2 font-bold">
                Legacy Farm No. 1 <Pill>you</Pill>
              </div>
              <dl className="mt-1.5 grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-sm">
                <dt className="text-muted">pay</dt>
                <dd className="m-0 text-right">8.00 cr/h</dd>
                <dt className="text-muted">hours</dt>
                <dd className="m-0 text-right">8 h</dd>
              </dl>
            </Tile>
          </More>
          <table className="mt-4 w-full border-collapse text-[15px]">
            <thead>
              <tr>
                <th className={TH}>Position</th>
                <th className={TH_NUM}>Pay</th>
                <th className={TH_NUM}>Hours</th>
              </tr>
            </thead>
            <tbody>
              <tr className="hover:bg-surface-2">
                <td className={TD}>
                  Legacy Farm No. 1 <Pill>you</Pill>
                </td>
                <td className={TD_NUM}>8.00 cr/h</td>
                <td className={TD_NUM}>8 h</td>
              </tr>
            </tbody>
          </table>
        </Card>
      </Stack>
    ),
  },
  {
    id: "market",
    title: "Order book, TimeSeries, Countdown, Meter",
    spec: "§7.15–7.17 — bids in the good fill, asks in the crit fill, labelled. Your own series is ink, 2px. Countdowns never shift the layout.",
    node: (
      <Two>
        <Card title="food" aside={<Pill>steady 3 days</Pill>} subtitle="Tabs below md; every panel at once from md.">
          <Tabs
            label="food book"
            layout="md:grid-cols-1"
            tabs={[
              {
                key: "book",
                label: "Book",
                node: (
                  <OrderBook
                    bids={[
                      { price: 130, qty: 40 },
                      { price: 128, qty: 25 },
                      { price: 125, qty: 60 },
                    ]}
                    asks={[
                      { price: 133, qty: 30 },
                      { price: 136, qty: 50 },
                      { price: 140, qty: 20 },
                    ]}
                  />
                ),
              },
              { key: "tape", label: "Tape", node: <p className="text-muted m-0 text-sm">Day 1, 11 AM · 1.31 · 3 · you bought from Legacy Farm No. 1</p> },
            ]}
          />
          <div className="mt-4 grid gap-2">
            <Meter label="Food" value={76} hint="Each hour you eat one Food from your pantry if there is any. Buy Food on the Market screen." />
            <Meter label="Shelter" value={40} />
            <Meter label="Comfort" value={14} />
          </div>
        </Card>
        <Card title="food · last 3 days" subtitle="price, cr">
          <TimeSeries ticks={ticks} series={series} />
          <p className="mt-3 mb-0 flex flex-wrap gap-4">
            <Countdown at={new Date(Date.now() + 41 * 60_000).toISOString()} label="next hour" />
            <WorldClock clock={{ epoch: 1, cycle: 12, tick: 17, ticks_per_cycle: 24 }} nextTickAt={new Date(Date.now() + 41 * 60_000).toISOString()} tickSeconds={3600} />
          </p>
          <p className="mt-2 mb-0">
            <WorldClock clock={{ epoch: 1, cycle: 12, tick: 17, ticks_per_cycle: 24 }} nextTickAt={null} tickSeconds={3600} />
          </p>
        </Card>
      </Two>
    ),
  },
  {
    id: "diff",
    title: "Diff since last seen",
    spec: "§7.13 — the same Feed, narrated one line per event.",
    node: (
      <Card title="While you were away" icon="clock" subtitle="since Day 1, 5 AM">
        <DiffSinceLastSeen events={events} t={(k) => ({ compensation: "Payslip", job: "Position" })[k] ?? k} me={41} />
      </Card>
    ),
  },
  {
    id: "overlays",
    title: "Sheet, Toast, Empty state",
    spec: "§7.19–7.20 — bottom sheet on phones, centred dialog from md. One toast at a time. Never a blank card.",
    node: (
      <Two>
        <Card title="Overlays">
          <SheetDemo />
        </Card>
        <Card title="Payslip" icon="coin">
          <p className="text-muted m-0">No payslip yet. The first comes at the end of the day.</p>
          <FooterStrip>
            <span>41 citizens</span>
            <span>1 people</span>
            <span>0 without work</span>
            <span>price index 1.00</span>
          </FooterStrip>
        </Card>
      </Two>
    ),
  },
  {
    id: "commune",
    title: "The Commune: the kit without money",
    spec: "GDD 15, S2.11 — the same components in a society with no currency: the TopBar with no balance, Num with an Explain in units and hours, the Ledger as a draw record and as the record of contribution, the Meter as a quorum bar and a shelf, the Diff narrating a ballot cast by default. Nothing here says cr.",
    node: (
      <Stack>
        <div className="border-line -mx-4 overflow-hidden border-y">
          <TopBar
            name="The Commune"
            preset="commune"
            clock={<WorldClock clock={shellClock} nextTickAt={shellNext} tickSeconds={3600} />}
            phoneClock={<WorldClock clock={shellClock} nextTickAt={shellNext} tickSeconds={3600} phone />}
            account={SHELL_ACCOUNT}
          />
          <ScreenNav items={COMMUNE_NAV} />
          <div className="bg-bg text-muted px-4 py-6 text-sm">The page, under the bars. No balance in the header: there is no money to count.</div>
          <TabBar items={COMMUNE_NAV} account={SHELL_ACCOUNT} inline />
        </div>
        <Two>
          <Card title="Your Household & the Store" aside={<Pill tone="attn">1 ballot waits</Pill>}>
            <Verdict parts={["You're fed, housed and working", { text: ", but a ballot waits for you tonight", tone: "attn" }, "."]} />
            <FactList
              items={[
                { key: "store", label: "Common Store", value: "31 food, 12 wares on the shelf", gloss: "you may draw 2 Food" },
                { key: "line", label: "Ledger of Contribution", value: "4 of 6 hours today", gloss: "norm met 3 of 4 days so far" },
                { key: "ballots", label: "Ballot", value: <span className="text-attn">2 proposals open</span>, gloss: "1 waits for your ballot" },
                { key: "dwelling", label: "Dwelling", value: "No. 17" },
              ]}
            />
            <p className="text-muted mt-3 mb-0 text-sm">
              A draw with its rule: <Num value="+2 food" explain={drawExplain} />. Hours on the record: <Num value="+4" unit="h" explain={normExplain} />.
            </p>
          </Card>
          <Card title="Draw record" icon="store" subtitle="Every draw, with the rule that served it. There is no wage.">
            <Ledger
              amountLabel="Drew"
              rows={[
                { key: "d2", epoch: 1, when: "Day 2, 6 AM", what: "Drew from the Store", goods: "+2 food", explain: drawExplain },
                { key: "d1", epoch: 1, when: "Day 1, 11 PM", what: "Surplus share at the day's end", goods: "+1 wares", explain: { ...drawExplain, rule: "store_surplus_share", formula: "surplus / active_citizens", result: { Int: 1 } } },
              ]}
            />
          </Card>
        </Two>
        <Two>
          <Card title="Your line" icon="ledger">
            <Verdict parts={["You have given 4 of the norm's 6 hours today at the workshop."]} />
            <Ledger
              amountLabel="Hours"
              rows={[
                { key: "t", epoch: 1, when: "Day 2, so far", what: "Short of the norm of 6 at the workshop", hours: 4 },
                { key: "y", epoch: 1, when: "Day 1", what: "Met the norm of 6 at the workshop", hours: 6 },
              ]}
            />
          </Card>
          <Card title="The shelves" icon="store">
            <Figures>
              <Figure label="food on the shelf" value="31" bar={<Meter label="food against what is asked" value={100} threshold={0} tone="good" bare />} status="12 asked this hour." />
              <Figure label="wares on the shelf" value="3" bar={<Meter label="wares against what is asked" value={38} threshold={0} tone="attn" bare />} tone="attn" status="8 asked, more than is here." />
            </Figures>
            <div className="mt-4 grid gap-2">
              <span className="text-muted text-xs font-bold tracking-caps uppercase">Quorum on proposal #3</span>
              <Meter label="Quorum" value={35} threshold={20} hint="7 of 20 eligible have cast; the quorum is 20 %." />
              <span className="text-good text-sm">Carries as it stands: 5 yes, 2 no.</span>
            </div>
          </Card>
        </Two>
        <Card title="While you were away" icon="clock" subtitle="since Day 1, 5 AM">
          <DiffSinceLastSeen events={communeEvents} t={(k) => ({ compensation: "Draw record", job: "Contribution", proposal: "Proposal" })[k] ?? k} me={41} />
        </Card>
      </Stack>
    ),
  },
];
