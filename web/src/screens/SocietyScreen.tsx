// Society (GDD 10, 14.1; TDD 13; S1.13; docs/style.md §10 Society): the
// Verdict — fed, in hardship, prices — the four figures that justify it, the
// rest of the stats this society keeps (the subset its capabilities select),
// the offices, the Chronicle read by day, the scoreboard its constitution
// names with your row pinned first, the citizens with their flags, and the
// published householder script as detail. Every headline links to its event.

import { useState, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { useOffices } from "../api/assembly";
import { credits } from "../api/client";
import { useCapabilities, useHome, useLexicon, useSociety } from "../api/hooks";
import { useChronicle, useCitizens, useHouseholders, useScoreboard, useStats } from "../api/civic";
import { ButtonLink } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { ChronicleReader } from "../components/ChronicleReader";
import { TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { More } from "../components/More";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { Markdown } from "../lib/markdown";
import { fieldName } from "../lib/policy";
import { StatTiles } from "../components/StatTiles";
import { societyVerdict } from "../lib/verdict";
import { dayOf, whenOfTick } from "../lib/when";

type Aggregates = Record<string, unknown>;

function Handle({ children, me }: { children: ReactNode; me: boolean }) {
  return (
    <>
      {children}
      {me ? (
        <Pill className="ml-2">
          you
        </Pill>
      ) : null}
    </>
  );
}

export function SocietyScreen({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const society = useSociety(id);
  const stats = useStats(id);
  const home = useHome(id);
  const [cycle, setCycle] = useState<number | null>(null);
  const chronicle = useChronicle(id, cycle);
  const scoreboard = useScoreboard(id);
  const citizens = useCitizens(id);
  const script = useHouseholders(id);
  // Who holds office (S2.8): read where there is governance; a non-citizen's 403 simply hides the tile.
  const offices = useOffices(id, caps.data !== undefined && caps.data.governance !== "none");

  if (caps.isPending || stats.isPending || society.isPending) return <p className="text-muted">Loading.</p>;
  if (caps.error || stats.error || society.error) return <p className="text-crit">Could not load: {String(caps.error ?? stats.error ?? society.error)}</p>;
  const c = caps.data!;
  const s = stats.data!;
  const byNorm = c.labor === "norm";
  const honors = c.governance !== "none";
  const current = society.data!.clock.cycle;
  const shown = chronicle.data?.cycle ?? cycle ?? current;
  const me = home.data?.citizen.id ?? -1;
  const a = (s.last_cycle ?? {}) as Aggregates;
  const display = society.data!.display;
  const verdict = societyVerdict({
    name: display,
    fed: typeof a.need_fulfillment_rate === "number" ? a.need_fulfillment_rate : null,
    hardship: typeof a.hardship_count === "number" ? a.hardship_count : null,
    population: s.live.population,
    price: c.money && s.live.price_index != null ? { now: s.live.price_index, yesterday: typeof a.price_index === "number" ? a.price_index : null } : null,
  });
  // Your row first (§10 Society, phone), then the top of the board as ranked.
  const rows = scoreboard.data?.rows ?? [];
  const pinned = [...rows.filter((r) => r.citizen === me), ...rows.filter((r) => r.citizen !== me).slice(0, 20)];

  return (
    <div>
      <PageHeader
        title={display}
        meta={
          <Link to="/public/s/$id" params={{ id: String(id) }}>
            the public view
          </Link>
        }
      />

      <Stack>
        <Card title={`How ${display} is doing`} icon="people">
          <Verdict parts={verdict} />
          <StatTiles stats={s} money={c.money} credit={c.contracts.includes("credit")} orgs={c.org_kinds.length > 0} t={t} />
        </Card>

        {honors && offices.data && offices.data.offices.length > 0 ? (
          <Card title={t("office")} icon="office" testId="offices-tile">
            <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-4">
              {offices.data.offices.map((o) => (
                <Tile key={o.kind} testId={`office-tile-${o.kind}`} className="grid content-start gap-1">
                  <span className="text-muted text-xs font-bold tracking-caps uppercase">
                    {fieldName(o.kind)} · {o.holders.length} of {o.seats}
                  </span>
                  {o.holders.length === 0 ? <span className="text-muted text-sm">Nobody sits.</span> : null}
                  {o.holders.map((h) => (
                    <span key={h.citizen}>
                      {h.handle}
                      <span className="text-muted text-sm"> through {dayOf(h.term_ends_cycle)}</span>
                    </span>
                  ))}
                  {o.election ? (
                    <Link to="/s/$id/assembly" params={{ id: String(id) }} className="text-sm">
                      election open, {o.election.candidates.length} {o.election.candidates.length === 1 ? "candidate" : "candidates"}
                    </Link>
                  ) : null}
                  {o.i_hold ? (
                    <ButtonLink to="/s/$id/coordinator" params={{ id: String(id) }} className="mt-1 justify-self-start">
                      Your workspace
                    </ButtonLink>
                  ) : null}
                </Tile>
              ))}
            </div>
          </Card>
        ) : null}

        <Two className="items-start">
          <Card title={t("chronicle")} icon="page">
            <ChronicleReader id={id} cycle={shown} setCycle={setCycle} current={current} headlines={chronicle.data?.headlines ?? []} pending={chronicle.isPending} link />
          </Card>
          <Card title={t("scoreboard")} icon="assembly" subtitle={`What "doing well" means is the constitution's claim, not ours; other societies keep other scores.`}>
            {scoreboard.data && rows.length > 0 ? (
              byNorm ? (
                // The Commune's claim (GDD 6.2): the contribution record and the assembly's
                // honors; the society-level need fulfillment is among the numbers above.
                <table className="w-full border-collapse text-[15px]" data-testid="scoreboard">
                  <thead>
                    <tr>
                      <th className={TH}>Citizen</th>
                      <th className={TH_NUM}>Hours given</th>
                      <th className={TH_NUM}>Norm met</th>
                      <th className={TH_NUM}>{t("honor")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pinned.map((r) => (
                      <tr key={r.citizen} className="hover:bg-surface-2">
                        <td className={TD}>
                          <Handle me={r.citizen === me}>{r.handle}</Handle>
                        </td>
                        <td className={TD_NUM}>{(r.contribution?.hours_total ?? 0).toFixed(0)}</td>
                        <td className={TD_NUM}>{r.contribution && r.contribution.days > 0 ? `${r.contribution.norm_met_days} of ${r.contribution.days} days` : "—"}</td>
                        <td className={TD_NUM}>{(r.honors ?? 0) > 0 ? r.honors : <span className="text-muted">—</span>}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <table className="w-full border-collapse text-[15px]" data-testid="scoreboard">
                  <thead>
                    <tr>
                      <th className={TH}>Citizen</th>
                      <th className={TH_NUM}>Net worth</th>
                      <th className={TH_NUM}>Self-made</th>
                      <th className={`${TH} hidden md:table-cell`}>Firms</th>
                      {honors ? <th className={TH_NUM}>{t("honor")}</th> : null}
                    </tr>
                  </thead>
                  <tbody>
                    {pinned.map((r) => {
                      const firms = r.firms.map((f) => `${f.name} (${credits(f.book_value)})`).join(", ");
                      return (
                        <tr key={r.citizen} className="hover:bg-surface-2">
                          <td className={TD}>
                            <Handle me={r.citizen === me}>{r.handle}</Handle>
                            {firms ? <span className="text-muted block text-sm md:hidden">{firms}</span> : null}
                          </td>
                          <td className={TD_NUM}>{credits(r.net_worth)}</td>
                          <td className={`${TD_NUM} ${r.self_made < 0 ? "text-crit" : ""}`}>{credits(r.self_made)}</td>
                          <td className={`${TD} hidden text-sm md:table-cell`}>{firms}</td>
                          {honors ? <td className={TD_NUM}>{(r.honors ?? 0) > 0 ? r.honors : <span className="text-muted">—</span>}</td> : null}
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              )
            ) : (
              <p className="text-muted m-0">{scoreboard.isPending ? "Loading." : "No scoreboard here: this society does not keep one."}</p>
            )}
          </Card>
        </Two>

        <Card title="Citizens" icon="people" aside={citizens.data ? <span className="text-muted text-sm font-normal">{citizens.data.citizens.length}</span> : null}>
          {citizens.data ? (
            <table className="w-full border-collapse text-[15px]" data-testid="citizens">
              <thead>
                <tr>
                  <th className={TH}>Handle</th>
                  <th className={`${TH} hidden md:table-cell`}>Kind</th>
                  <th className={`${TH} hidden md:table-cell`}>Since</th>
                  {honors ? <th className={TH_NUM}>{t("honor")}</th> : null}
                  <th className={TH}>Flags</th>
                </tr>
              </thead>
              <tbody>
                {citizens.data.citizens.map((z) => {
                  const flags = Object.entries(z.flags as Record<string, boolean>)
                    .filter(([, v]) => v)
                    .map(([k]) => k.replace(/_/g, " "));
                  const kind = `${z.kind}${z.dormant ? ", dormant" : ""}`;
                  return (
                    <tr key={z.id} className={`hover:bg-surface-2 ${z.dormant ? "text-muted" : ""}`}>
                      <td className={TD}>
                        <Link to="/s/$id/talk/$channel" params={{ id: String(id), channel: `dm:${z.id}` }}>
                          {z.handle}
                        </Link>
                        {z.id === me ? (
                          <Pill className="ml-2">
                            you
                          </Pill>
                        ) : null}
                        <span className="text-muted block text-sm md:hidden">
                          {kind} · since {whenOfTick(z.joined_tick)}
                        </span>
                      </td>
                      <td className={`${TD} hidden md:table-cell`}>{kind}</td>
                      <td className={`${TD} hidden tabular-nums md:table-cell`}>{whenOfTick(z.joined_tick)}</td>
                      {honors ? <td className={TD_NUM}>{(z.honors ?? 0) > 0 ? z.honors : <span className="text-muted">—</span>}</td> : null}
                      <td className={TD}>
                        {flags.map((f) => (
                          <Pill key={f} tone={/hardship|destitut|default|flag/.test(f) ? "crit" : "neutral"} className="mr-1">
                            {f}
                          </Pill>
                        ))}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          ) : (
            <p className="text-muted m-0">Loading.</p>
          )}
          <More summary="The script householders follow" testId="householder-more">
            <div className="max-w-prose" data-testid="householder-script">
              {script.data ? <Markdown text={script.data.markdown} /> : <p className="text-muted">Loading.</p>}
            </div>
          </More>
        </Card>

        <FooterStrip>
          <span>{s.live.population} citizens</span>
          <span>{s.live.active_humans} people</span>
          <span>{s.live.unemployed} without work</span>
          {c.money && s.live.price_index != null ? (
            <span>
              {t("society_stat").toLowerCase()} {s.live.price_index.toFixed(2)}
            </span>
          ) : null}
        </FooterStrip>
      </Stack>
    </div>
  );
}
