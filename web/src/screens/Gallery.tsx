// A Storybook-free gallery of the shared components with fixed data, for
// visual checks (TDD 11, S1.7).

import type { EventRef } from "../api/client";
import { Countdown } from "../components/Countdown";
import { DiffSinceLastSeen } from "../components/DiffSinceLastSeen";
import { Ledger } from "../components/Ledger";
import { Meter } from "../components/Meter";
import { Num } from "../components/Num";
import { OrderBook } from "../components/OrderBook";
import { TimeSeries } from "../components/TimeSeries";
import { WorldClock } from "../components/WorldClock";

const explain = {
  rule: "payroll_hourly",
  inputs: [
    ["tick_hours", 192],
    ["wage", 7.8],
  ] as [string, unknown][],
  formula: "wage x tick_hours / 24",
  result: 62.4,
};

const ticks = Array.from({ length: 48 }, (_, i) => i);
const series = [
  { label: "Food", values: ticks.map((i) => 1.3 + 0.05 * Math.sin(i / 4)) },
  { label: "Wares", values: ticks.map((i) => 3.6 + 0.2 * Math.cos(i / 7)) },
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

export function Gallery() {
  const t = (k: string) => ({ compensation: "Payslip", job: "Position" })[k] ?? k;
  return (
    <div className="flex flex-col gap-10">
      <section>
        <h2 className="text-xl">Num with Explain</h2>
        <p className="mt-2">
          Last payslip: <Num value="62.40" unit="cr" explain={explain} />
        </p>
      </section>
      <section>
        <h2 className="text-xl">Meter</h2>
        <div className="mt-2 flex max-w-md flex-col gap-2">
          <Meter label="Food" value={76} hint="Each hour you eat one Food from your pantry if there is any. Buy Food on the Market screen." />
          <Meter label="Shelter" value={100} />
          <Meter label="Comfort" value={14} />
        </div>
      </section>
      <section>
        <h2 className="text-xl">Ledger</h2>
        <div className="mt-2 max-w-lg">
          <Ledger
            rows={[
              { key: "1", when: "c11 end", what: "Payslip, Iron & Sons", cents: 6240, explain },
              { key: "2", when: "c11 end", what: "Rent, Dwelling #17", cents: -800 },
              { key: "3", when: "c12 t3", what: "Bought 2 food", goods: "2 food" },
            ]}
          />
        </div>
      </section>
      <section>
        <h2 className="text-xl">Order book</h2>
        <div className="mt-2">
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
        </div>
      </section>
      <section>
        <h2 className="text-xl">Time series</h2>
        <div className="mt-2 max-w-xl">
          <TimeSeries ticks={ticks} series={series} />
        </div>
      </section>
      <section>
        <h2 className="text-xl">Diff since last seen</h2>
        <div className="mt-2 max-w-lg">
          <DiffSinceLastSeen events={events} t={t} me={41} />
        </div>
      </section>
      <section>
        <h2 className="text-xl">World clock</h2>
        <p className="mt-2">
          <WorldClock clock={{ epoch: 1, cycle: 12, tick: 17, ticks_per_cycle: 24 }} nextTickAt={new Date(Date.now() + 41 * 60_000).toISOString()} tickSeconds={3600} />
        </p>
        <p className="mt-2">
          <WorldClock clock={{ epoch: 1, cycle: 12, tick: 17, ticks_per_cycle: 24 }} nextTickAt={null} tickSeconds={3600} />
        </p>
      </section>
      <section>
        <h2 className="text-xl">Countdown</h2>
        <p className="mt-2">
          <Countdown at={new Date(Date.now() + 41 * 60_000).toISOString()} label="Next tick" />
        </p>
      </section>
    </div>
  );
}
