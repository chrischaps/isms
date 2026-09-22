// The Ledger of Contribution (GDD 6.2, 15; S2.7; docs/style.md §10 by
// analogy): every citizen's hours, exactly, and their output as the
// society's monitoring attributes it, with the sigma stated so nobody
// mistakes an estimate for a count; the norm, met or not, today and over
// every day on the record; the assembly's honors. The Verdict is your own
// line — what you have given today, against the norm — and your row is
// marked. Nobody can be made to work, and everyone can see what you did.
// Mounted where `caps.labor` is "norm".

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useLedger, type ContributionRow } from "../api/commons";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { ButtonLink } from "../components/Button";
import { Card, Stack } from "../components/Card";
import { Figure, Figures } from "../components/Figure";
import { Ledger, TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { More } from "../components/More";
import { PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { contributionRows } from "../lib/contribution";
import { useNames, type Names } from "../lib/names";
import { ledgerVerdict } from "../lib/verdict";

function hours(h: number): string {
  return Number.isInteger(h) ? String(h) : h.toFixed(1);
}

/** One citizen's line: three columns on a phone, the rest folded under the name (§3). */
function Row({ r, norm, names, sigma }: { r: ContributionRow; norm: number | null; names: Names; sigma: number }) {
  const est = sigma > 0 ? "~" : "";
  const workplaces = r.workplaces.map((w) => names.workplaceTitle(w)).join(", ");
  return (
    <tr className={`hover:bg-surface-2 ${r.dormant ? "text-muted" : ""}`} data-testid={r.is_me ? "ledger-row-me" : `ledger-row-${r.citizen}`}>
      <td className={TD}>
        {r.handle}
        {r.is_me ? (
          <>
            {" "}
            <Pill>you</Pill>
          </>
        ) : null}
        {r.dormant ? <span className="text-muted text-sm"> · away</span> : null}
        {workplaces ? <span className="text-muted block text-sm">{workplaces}</span> : null}
        <span className="text-muted block text-sm md:hidden">
          yesterday {hours(r.hours_yesterday)} h, {est}
          {r.attributed_yesterday.toFixed(1)} out · norm met {r.days === 0 ? "—" : `${r.norm_met_days} of ${r.days}`} · {hours(r.hours_total)} h ever
          {r.honors > 0 ? ` · ${r.honors} ${r.honors === 1 ? "honor" : "honors"}` : ""}
        </span>
      </td>
      <td className={TD_NUM} data-testid="hours-today">
        {hours(r.hours_today)}
        {norm != null ? (
          <>
            {" "}
            <span className={`block text-sm ${r.norm_met_today ? "text-good" : "text-muted"}`}>{r.norm_met_today ? "met" : `of ${norm}`}</span>
          </>
        ) : null}
      </td>
      <td className={TD_NUM}>
        {est}
        {r.attributed_today.toFixed(1)}
      </td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{hours(r.hours_yesterday)}</td>
      <td className={`${TD_NUM} hidden md:table-cell`}>
        {est}
        {r.attributed_yesterday.toFixed(1)}
      </td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{r.days === 0 ? <span className="text-muted">—</span> : `${r.norm_met_days} of ${r.days}`}</td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{hours(r.hours_total)}</td>
      <td className={`${TD_NUM} hidden md:table-cell`}>{r.honors > 0 ? r.honors : <span className="text-muted">—</span>}</td>
    </tr>
  );
}

export function LedgerScreen({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const byNorm = caps.data?.labor === "norm";
  const ledger = useLedger(id, byNorm);
  const names = useNames(id, home.data?.citizen.id, home.data !== undefined);

  if (caps.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error) return <p className="text-crit">Could not load: {String(caps.error)}</p>;
  // No norm, no Ledger: said before anyone is asked to join.
  if (!byNorm) {
    return <p className="text-muted">There is no Ledger of Contribution in this society: labor here is not by norm.</p>;
  }
  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }}>
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-crit">Could not load: {String(home.error)}</p>;
  if (ledger.isPending) return <p className="text-muted">Loading.</p>;
  if (ledger.error) return <p className="text-crit">Could not load: {String(ledger.error)}</p>;
  const v = ledger.data!;
  const h = home.data!;
  const norm = v.norm_hours ?? null;
  const active = v.rows.filter((r) => !r.dormant);
  const away = v.rows.filter((r) => r.dormant);
  const me = v.rows.find((r) => r.is_me);
  const metToday = active.filter((r) => r.norm_met_today).length;
  const verdict = me
    ? ledgerVerdict({
        norm,
        hoursToday: me.hours_today,
        normMet: me.norm_met_today,
        workplaces: me.workplaces.map((w) => names.workplaceTitle(w)),
        days: me.days,
        metDays: me.norm_met_days,
        leastStaffed: v.least_staffed != null ? names.workplace(v.least_staffed) : null,
      })
    : ["You are not on the record yet."];

  return (
    <div>
      <PageHeader
        title={t("ledger_screen")}
        meta={
          <span data-testid="ledger-rule">
            {norm != null ? (
              <>
                The norm asks <b>{norm} hours</b> a day of everyone.{" "}
              </>
            ) : (
              "No norm is published. "
            )}
            Output is attributed under <b>{v.monitoring}</b> monitoring
            {v.sigma > 0 ? <> (σ {v.sigma.toFixed(2)}): the ~ figures are estimates. Hours are exact.</> : ": the figures are exact."}
          </span>
        }
      />
      <Stack>
        <Card title="Your line" icon="ledger">
          <div data-testid="my-line">
            <Verdict parts={verdict} />
          </div>
          {me ? (
            <Figures className="mb-3">
              <Figure label="Hours today" value={hours(me.hours_today)} unit={norm != null ? `/ ${norm}` : "h"} tone={norm != null && !me.norm_met_today && me.hours_today === 0 ? "attn" : "good"} status={norm != null ? (me.norm_met_today ? "The norm is met." : `${hours(Math.max(0, norm - me.hours_today))} more meet the norm.`) : "Counted, not estimated."} />
              <Figure label="Output today" value={`${v.sigma > 0 ? "~" : ""}${me.attributed_today.toFixed(1)}`} status={v.sigma > 0 ? "An estimate under this monitoring." : "Exact under this monitoring."} />
              <Figure label="Norm met" value={me.days === 0 ? "—" : String(me.norm_met_days)} unit={me.days === 0 ? undefined : `/ ${me.days} days`} status={me.days === 0 ? "No day on the record yet." : "Over every day on the record."} />
              <Figure label="Honors" value={String(me.honors)} status={me.honors > 0 ? "From the assembly." : "None yet."} />
            </Figures>
          ) : null}
          <ButtonLink to="/s/$id/work" params={{ id: String(id) }}>
            {me && me.workplaces.length === 0 ? "Take a position" : "Adjust your hours"}
          </ButtonLink>
          {me ? (
            // The record as a ledger (§7.14, S2.11): hours are the unit here, as
            // credits are on a payslip. The wire carries two days of it (Q157).
            <More summary="Your record, day by day" testId="my-record-more">
              <div data-testid="my-record">
                <Ledger
                  rows={contributionRows(me, norm, h.clock.cycle - 1, h.clock.epoch - 1, me.workplaces[0] !== undefined ? names.workplaceTitle(me.workplaces[0]) : undefined)}
                  amountLabel="Hours"
                />
              </div>
            </More>
          ) : null}
        </Card>

        <Card
          title="Everyone"
          icon="people"
          aside={
            norm != null ? (
              <span className="text-muted text-sm font-normal tabular-nums" data-testid="met-today">
                {metToday} of {active.length} at the norm so far today
              </span>
            ) : null
          }
        >
          <table className="w-full border-collapse text-[15px]" data-testid="ledger">
            <thead>
              <tr>
                <th className={TH}>Citizen</th>
                <th className={TH_NUM}>Hours today</th>
                <th className={TH_NUM}>Output today</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Hours yesterday</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Output yesterday</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Norm met</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Hours ever</th>
                <th className={`${TH_NUM} hidden md:table-cell`}>Honors</th>
              </tr>
            </thead>
            <tbody>
              {active.map((r) => (
                <Row key={r.citizen} r={r} norm={norm} names={names} sigma={v.sigma} />
              ))}
              {away.length > 0 ? (
                <tr>
                  <td colSpan={8} className="text-muted border-line border-b pt-4 pb-1 text-xs font-bold tracking-caps uppercase">
                    Away
                  </td>
                </tr>
              ) : null}
              {away.map((r) => (
                <Row key={r.citizen} r={r} norm={norm} names={names} sigma={v.sigma} />
              ))}
            </tbody>
          </table>
          <p className="text-muted mt-3 mb-0 text-sm">
            Hours are what each citizen chose to give; they are counted, not estimated. Output is what the workplace could attribute to them under the monitoring the
            assembly keeps: with no foreman the figure is noisy by design, and whether to meter it exactly is a policy the assembly can vote. The record closes at the end
            of every day; nobody can be fired for it, and nobody can be made to work.
          </p>
        </Card>
      </Stack>
    </div>
  );
}
