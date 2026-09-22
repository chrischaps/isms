// StatTiles (docs/style.md §10 Society): the numbers a society keeps, chosen
// by capability — money systems get prices and wages, credit systems the
// credit outstanding, everyone the needs figures. Four glance Figures with
// status lines, the rest as a FactList, the Gini and the definitions as
// detail. Shared by the Society screen and the public view.

import { credits } from "../api/client";
import { FactList, type Fact } from "./FactList";
import { Figure, Figures } from "./Figure";
import { More } from "./More";
import { fedTone, needTone } from "../lib/verdict";

type Aggregates = Record<string, unknown>;

function num(v: unknown, digits = 0): string {
  return typeof v === "number" ? v.toFixed(digits) : "—";
}

export function StatTiles({
  stats,
  money,
  credit,
  orgs,
  t,
}: {
  stats: { live: { population: number; active_humans: number; unemployed: number; price_index?: number | null }; firm_count: number; credit_outstanding: number; last_cycle?: Aggregates | null };
  money: boolean;
  credit: boolean;
  orgs: boolean;
  t: (k: string) => string;
}) {
  const a = stats.last_cycle ?? {};
  const live = stats.live;
  const fed = typeof a.need_fulfillment_rate === "number" ? a.need_fulfillment_rate : null;
  const hardship = typeof a.hardship_count === "number" ? a.hardship_count : null;
  const wellbeing = typeof a.median_wellbeing === "number" ? a.median_wellbeing : null;
  const price = live.price_index ?? (typeof a.price_index === "number" ? a.price_index : null);
  const facts: Fact[] = [
    { key: "citizens", label: "Citizens", value: String(live.population), gloss: `${live.active_humans} people here` },
    { key: "unemployed", label: "Without work", value: String(live.unemployed) },
  ];
  if (money) facts.push({ key: "wage", label: "Mean daily wage", value: typeof a.mean_cycle_wage === "number" ? `${a.mean_cycle_wage.toFixed(2)} cr` : "—", gloss: "yesterday, those paid anything" });
  if (orgs) facts.push({ key: "firms", label: "Firms", value: String(stats.firm_count) });
  if (credit) facts.push({ key: "credit", label: "Credit outstanding", value: `${credits(stats.credit_outstanding)} cr` });
  return (
    <div data-testid="stat-tiles">
      <Figures>
        <Figure
          label="Need fulfillment"
          value={fed === null ? "—" : `${(fed * 100).toFixed(0)}`}
          unit={fed === null ? undefined : "%"}
          tone={fedTone(fed)}
          status={fed === null ? "After the first day." : fed >= 0.9 ? "Nearly every citizen-day above the line." : fed >= 0.5 ? "Many days under the line." : "Most days under the line."}
        />
        <Figure
          label="In hardship"
          value={hardship === null ? "—" : String(hardship)}
          tone={hardship === null || hardship === 0 ? "good" : hardship * 10 >= live.population ? "crit" : "attn"}
          status={hardship === null ? "After the first day." : hardship === 0 ? "Nobody, at the end of yesterday." : "At the end of yesterday."}
        />
        {money ? <Figure label={t("society_stat")} value={num(price, 2)} status="Reference basket, Food = 1." /> : null}
        <Figure
          label="Median wellbeing"
          value={wellbeing === null ? "—" : String(Math.round(wellbeing))}
          unit={wellbeing === null ? undefined : "/ 100"}
          tone={wellbeing === null ? "good" : needTone(wellbeing)}
          status={wellbeing === null ? "After the first day." : "The middle citizen's needs, averaged."}
        />
      </Figures>
      <FactList className="mt-4" items={facts} />
      <More summary="About these numbers">
        <FactList items={[{ key: "gini", label: "Consumption Gini", value: num(a.consumption_gini, 2), gloss: "of what is eaten, worn and housed; never of wealth" }]} />
        <p className="text-muted mt-3 mb-0 max-w-prose text-sm">
          Need fulfillment is the share of citizen-days that never went under the hardship line. Hardship counts citizens whose Food stayed under the line for a
          whole day. Wellbeing is a citizen&apos;s three needs averaged; the median is the middle citizen&apos;s.
        </p>
      </More>
    </div>
  );
}

