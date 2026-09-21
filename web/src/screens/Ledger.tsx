// The Ledger of Contribution (GDD 6.2, 15; S2.7): every citizen's hours,
// exactly, and their output as the society's monitoring attributes it, with
// the sigma stated so nobody mistakes an estimate for a count; the norm, met
// or not, today and over every day on the record; the assembly's honors.
// Your row is marked. Nobody can be made to work, and everyone can see what
// you did. Mounted where `caps.labor` is "norm".

import { Link } from "@tanstack/react-router";
import { ApiError } from "../api/client";
import { useLedger, type ContributionRow } from "../api/commons";
import { useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useNames, type Names } from "../lib/names";

function hours(h: number): string {
  return Number.isInteger(h) ? String(h) : h.toFixed(1);
}

function Row({ r, norm, names, sigma }: { r: ContributionRow; norm: number | null; names: Names; sigma: number }) {
  const cls = r.is_me ? "text-ink bg-paper-2" : r.dormant ? "text-muted" : "";
  return (
    <tr className={`rule ${cls}`} data-testid={r.is_me ? "ledger-row-me" : `ledger-row-${r.citizen}`}>
      <td className="py-1 pr-2 align-top">
        {r.is_me ? "you" : r.handle}
        {r.dormant ? <span className="text-muted text-xs"> · away</span> : null}
        {r.workplaces.length > 0 ? <span className="text-muted block text-xs">{r.workplaces.map((w) => names.workplaceTitle(w)).join(", ")}</span> : null}
      </td>
      <td className="num py-1 pr-2 text-right align-top" data-testid="hours-today">
        {hours(r.hours_today)}
        {norm != null ? <span className={`text-xs ${r.norm_met_today ? "text-good" : "text-muted"}`}> {r.norm_met_today ? "met" : `of ${norm}`}</span> : null}
      </td>
      <td className="num py-1 pr-2 text-right align-top">{sigma > 0 ? "~" : ""}{r.attributed_today.toFixed(1)}</td>
      <td className="num py-1 pr-2 text-right align-top">{hours(r.hours_yesterday)}</td>
      <td className="num py-1 pr-2 text-right align-top">{sigma > 0 ? "~" : ""}{r.attributed_yesterday.toFixed(1)}</td>
      <td className="num py-1 pr-2 text-right align-top">
        {r.days === 0 ? <span className="text-muted">—</span> : `${r.norm_met_days} of ${r.days}`}
      </td>
      <td className="num py-1 pr-2 text-right align-top">{hours(r.hours_total)}</td>
      <td className="num py-1 text-right align-top">{r.honors > 0 ? r.honors : <span className="text-muted">—</span>}</td>
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
  if (caps.error) return <p className="text-bad">Could not load: {String(caps.error)}</p>;
  // No norm, no Ledger: said before anyone is asked to join.
  if (!byNorm) {
    return <p className="text-muted">There is no Ledger of Contribution in this society: labor here is not by norm.</p>;
  }
  if (home.isPending) return <p className="text-muted">Loading.</p>;
  if (home.error instanceof ApiError && home.error.status === 403) {
    return (
      <p className="text-muted">
        Join first, from the{" "}
        <Link to="/s/$id" params={{ id: String(id) }} className="underline">
          {t("home_title")}
        </Link>{" "}
        screen.
      </p>
    );
  }
  if (home.error) return <p className="text-bad">Could not load: {String(home.error)}</p>;
  if (ledger.isPending) return <p className="text-muted">Loading.</p>;
  if (ledger.error) return <p className="text-bad">Could not load: {String(ledger.error)}</p>;
  const v = ledger.data!;
  const norm = v.norm_hours ?? null;
  const active = v.rows.filter((r) => !r.dormant);
  const away = v.rows.filter((r) => r.dormant);
  const me = v.rows.find((r) => r.is_me);
  const metToday = active.filter((r) => r.norm_met_today).length;

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">{t("ledger")}</h2>
        <p className="text-muted text-sm" data-testid="ledger-rule">
          {norm != null ? (
            <>
              The norm asks <span className="text-ink">{norm} hours</span> a day of everyone.{" "}
            </>
          ) : (
            "No norm is published. "
          )}
          Output is attributed under <span className="text-ink">{v.monitoring}</span> monitoring
          {v.sigma > 0 ? (
            <>
              {" "}
              (σ {v.sigma.toFixed(2)}): the ~ figures are estimates. Hours are exact.
            </>
          ) : (
            ": the figures are exact."
          )}
        </p>
      </header>

      {me ? (
        <p className="text-sm" data-testid="my-line">
          {me.workplaces.length === 0 ? (
            <>
              You hold no position today.{" "}
              <Link to="/s/$id/work" params={{ id: String(id) }} className="underline">
                Take one
              </Link>
              {v.least_staffed != null ? <> ; labor is scarcest at the {names.workplace(v.least_staffed)}.</> : "."}
            </>
          ) : (
            <>
              You have given <span className="num">{hours(me.hours_today)}</span> hours today
              {norm != null ? (me.norm_met_today ? ", the norm met" : ` of the norm's ${norm}`) : ""}, at the {me.workplaces.map((w) => names.workplaceTitle(w)).join(" and ")}.
              {me.days > 0 ? ` Over ${me.days} ${me.days === 1 ? "day" : "days"} on the record you met the norm ${me.norm_met_days} ${me.norm_met_days === 1 ? "time" : "times"}.` : ""}{" "}
              <Link to="/s/$id/work" params={{ id: String(id) }} className="text-muted underline">
                adjust
              </Link>
            </>
          )}
        </p>
      ) : null}

      <section>
        <div className="flex flex-wrap items-baseline justify-between gap-3">
          <h3 className="text-lg">Everyone</h3>
          {norm != null ? (
            <span className="num text-muted text-sm" data-testid="met-today">
              {metToday} of {active.length} at the norm so far today
            </span>
          ) : null}
        </div>
        <table className="mt-2 w-full text-sm" data-testid="ledger">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Citizen</th>
              <th className="py-1 text-right font-normal">Hours today</th>
              <th className="py-1 text-right font-normal">Output today</th>
              <th className="py-1 text-right font-normal">Hours yesterday</th>
              <th className="py-1 text-right font-normal">Output yesterday</th>
              <th className="py-1 text-right font-normal">Norm met</th>
              <th className="py-1 text-right font-normal">Hours ever</th>
              <th className="py-1 text-right font-normal">Honors</th>
            </tr>
          </thead>
          <tbody>
            {active.map((r) => (
              <Row key={r.citizen} r={r} norm={norm} names={names} sigma={v.sigma} />
            ))}
            {away.length > 0 ? (
              <tr>
                <td colSpan={8} className="text-muted border-line border-t-2 pt-3 pb-1 text-xs uppercase tracking-wide">
                  Away
                </td>
              </tr>
            ) : null}
            {away.map((r) => (
              <Row key={r.citizen} r={r} norm={norm} names={names} sigma={v.sigma} />
            ))}
          </tbody>
        </table>
        <p className="text-muted mt-3 max-w-prose text-xs">
          Hours are what each citizen chose to give; they are counted, not estimated. Output is what the workplace could attribute to them under the monitoring the
          assembly keeps: with no foreman the figure is noisy by design, and whether to meter it exactly is a policy the assembly can vote. The record closes at the end
          of every day; nobody can be fired for it, and nobody can be made to work.
        </p>
      </section>
    </div>
  );
}
