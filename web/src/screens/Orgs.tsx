// Organizations (GDD 6.1 firms, 9.2 office archetype; S1.11; docs/style.md
// §10 Organizations): the Verdict — who is hiring and where you work — the
// firms you have a part in, the public job board with "Take it", the
// found-a-firm card with the slot picker showing scarcity and the cost
// against your balance and pantry, and every organization as a table.
// Kinds come from capabilities.

import { useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { ApiError, credits } from "../api/client";
import { useBoard, useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useAcceptOffer } from "../api/society";
import { useFoundOrg, useOrgsView, type OrgView } from "../api/orgs";
import { Button, ButtonRow } from "../components/Button";
import { Card, Stack, Tile, Two } from "../components/Card";
import { Field, Input, Select } from "../components/Field";
import { TD, TD_NUM, TH, TH_NUM } from "../components/Ledger";
import { FooterStrip, PageHeader } from "../components/PageHeader";
import { Pill } from "../components/Pill";
import { Verdict } from "../components/Verdict";
import { useNames } from "../lib/names";
import { jobLine } from "../lib/offers";
import { orgsVerdict } from "../lib/verdict";

const WORKPLACE_KINDS = ["farm", "mine", "foundry", "mill", "workshop", "machine_shop", "builder"];

function holding(o: OrgView, me: number): string | null {
  const own = o.ownership as Record<string, unknown>;
  const shares = own.shares as { issued: number; holdings: Record<string, number> } | undefined;
  if (shares && o.my_shares > 0) {
    return `${o.my_shares} of ${shares.issued} shares (${((100 * o.my_shares) / shares.issued).toFixed(0)} %)`;
  }
  if (o.members.includes(me)) return "member";
  return null;
}

export function Orgs({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const who = useNames(id, home.data?.citizen.id, home.data !== undefined);
  const orgs = useOrgsView(id);
  const board = useBoard(id);
  const found = useFoundOrg(id);
  const accept = useAcceptOffer(id);
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<string | null>(null);
  const [wpKind, setWpKind] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [taken, setTaken] = useState<string | null>(null);
  const [jobError, setJobError] = useState<string | null>(null);

  if (home.isPending || orgs.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
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
  if (home.error || orgs.error || caps.error) {
    return <p className="text-crit">Could not load: {String(home.error ?? orgs.error ?? caps.error)}</p>;
  }
  const h = home.data!;
  const v = orgs.data!;
  const c = caps.data!;
  const me = h.citizen.id;
  const byNorm = c.labor === "norm";
  // A posting with places left stays on the board after you take one of them.
  const myWorkplaces = new Set(
    h.labor.employment.map((k) => Number(((k.body as Record<string, unknown>).employment as Record<string, unknown> | undefined)?.workplace)),
  );
  const names = new Map(v.orgs.map((o) => [o.id, o.name]));
  const orgName = (oid: number) => names.get(oid) ?? who.org(oid);
  const kinds = c.org_kinds;
  const chosenKind = kind ?? kinds[0] ?? "firm";
  const slots = v.slots as Record<string, { total?: number | null; free?: number | null }>;
  const materialsHeld = (h.household.pantry as Record<string, number>).materials ?? 0;
  const canPayMoney = !c.money || h.household.balance >= v.founding.money;
  const canPayMaterials = wpKind === null || materialsHeld >= v.founding.materials;
  const slotFree = wpKind === null || slots[wpKind]?.total == null || (slots[wpKind]?.free ?? 0) > 0;
  const jobs = (board.data?.offers ?? []).map((o) => ({ o, line: jobLine(o) })).filter((j) => j.line !== null);
  const mine = v.orgs.filter((o) => o.i_manage || o.my_shares > 0 || o.members.includes(me));
  // Where you work: by contract, or under the norm by position.
  const workAt = [...new Set((h.labor.positions ?? []).map((p) => p.org))].map(orgName);
  const verdict = orgsVerdict({
    hiring: new Set(jobs.map((j) => j.line!.org)).size,
    places: jobs.reduce((n, j) => n + j.line!.places, 0),
    workAt,
    manage: mine.filter((o) => o.i_manage).map((o) => o.name),
    byNorm,
  });
  const cannotFound =
    name.trim().length === 0
      ? "name it first"
      : !canPayMoney
        ? `costs ${credits(v.founding.money)} cr — you have ${credits(h.household.balance)}`
        : !canPayMaterials
          ? `needs ${v.founding.materials} Materials — you hold ${materialsHeld}`
          : !slotFree
            ? "no slot free for that workplace"
            : undefined;

  const submit = () => {
    setError(null);
    found.mutate(
      { kind: chosenKind, name: name.trim(), first_workplace: wpKind ? { kind: wpKind } : null },
      {
        onSuccess: (r) => {
          const ev = r.events.find((e) => e.kind === "OrgFounded");
          const org = Number((ev?.payload as Record<string, Record<string, unknown>> | undefined)?.OrgFounded?.org);
          if (Number.isFinite(org)) {
            void navigate({ to: "/s/$id/orgs/$oid", params: { id: String(id), oid: String(org) } });
          }
        },
        onError: (e) => setError(e.message),
      },
    );
  };

  return (
    <div>
      <PageHeader
        title="Organizations"
        meta={
          <>
            <span>
              <b>{v.orgs.length}</b> in all
            </span>
            {mine.length > 0 ? (
              <span>
                <b>{mine.length}</b> yours
              </span>
            ) : null}
          </>
        }
      />

      <Stack>
        <Card title="Where you stand" icon="org" testId="orgs-standing">
          <Verdict parts={verdict} />
          <div className="grid gap-3 @min-[620px]:grid-cols-2">
            <Tile testId="my-positions" className="grid content-start gap-1">
              <span className="font-bold">Your {byNorm ? "positions" : `${t("job").toLowerCase()}s`}</span>
              {(h.labor.positions ?? []).length === 0 ? (
                <p className="text-muted m-0 text-sm">{byNorm ? "None yet; take one on the Work screen." : "None yet; the board is beside this."}</p>
              ) : (
                <ul className="m-0 grid list-none gap-1 p-0 text-sm">
                  {(h.labor.positions ?? []).map((p) => (
                    <li key={p.workplace}>
                      <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(p.org) }}>
                        {orgName(p.org)}
                      </Link>
                      <span className="text-muted"> · {who.workplaceTitle(p.workplace)}</span>
                    </li>
                  ))}
                </ul>
              )}
              <Link to="/s/$id/work" params={{ id: String(id) }} className="text-sm">
                Hours are set on {t("work_screen")}
              </Link>
            </Tile>
            <Tile testId="my-orgs-tile" className="grid content-start gap-1">
              <span className="font-bold">Firms you have a part in</span>
              {mine.length === 0 ? (
                <p className="text-muted m-0 text-sm">None yet. Found one below, or buy shares on the Market.</p>
              ) : (
                <ul className="m-0 grid list-none gap-1 p-0 text-sm" data-testid="my-orgs">
                  {mine.map((o) => (
                    <li key={o.id} className="flex flex-wrap items-baseline justify-between gap-x-3">
                      <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(o.id) }}>
                        {o.name}
                      </Link>
                      <span className="text-muted">
                        {o.kind.replace("_", " ")}
                        {o.i_manage ? " · you manage it" : ""}
                        {holding(o, me) ? ` · ${holding(o, me)}` : ""}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Tile>
          </div>
        </Card>

        <Two>
          <Card
            title="Job board"
            icon="work"
            testId="job-board-card"
            subtitle={`Open ${t("job").toLowerCase()}s from the notice board. Taking one sets nothing else; hours live on ${t("work_screen")}.`}
          >
            {jobs.length === 0 ? (
              <p className="text-muted m-0">Nobody is hiring this hour.</p>
            ) : (
              <ul className="m-0 grid list-none gap-3 p-0" data-testid="job-board">
                {jobs.map(({ o, line }) => (
                  <li key={o.id}>
                    <Tile className="grid gap-2">
                      <div className="flex flex-wrap items-baseline justify-between gap-x-3">
                        <span>
                          <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(line!.org) }} className="font-bold">
                            {orgName(line!.org)}
                          </Link>
                          <span className="text-muted block text-sm">{who.workplaceTitle(line!.workplace)}</span>
                        </span>
                        <span className="font-display font-bold tabular-nums">{line!.pay}</span>
                      </div>
                      <p className="text-muted m-0 text-sm">
                        up to {line!.hours} h a day · {line!.term} · notice {line!.notice} day(s) · {line!.places} open
                      </p>
                      {myWorkplaces.has(line!.workplace) ? (
                        <Pill className="justify-self-start">you work here</Pill>
                      ) : (
                        <Button
                          disabled={accept.isPending}
                          className="justify-self-start"
                          onClick={() => {
                            setJobError(null);
                            setTaken(null);
                            accept.mutate(o.id, {
                              onSuccess: () => setTaken(who.workplace(line!.workplace)),
                              onError: (e) => setJobError(e.message),
                            });
                          }}
                        >
                          Take it
                        </Button>
                      )}
                    </Tile>
                  </li>
                ))}
              </ul>
            )}
            {taken ? (
              <p className="mt-3 text-sm" role="status" data-testid="job-taken">
                Taken: {taken}. Your hours there are 0 until you set them on{" "}
                <Link to="/s/$id/work" params={{ id: String(id) }}>
                  {t("work_screen")}
                </Link>
                .
              </p>
            ) : null}
            {jobError ? (
              <p className="text-crit mt-3 text-sm" role="alert">
                {jobError}
              </p>
            ) : null}
          </Card>

          {kinds.length > 0 ? (
            <Card title="Found one" icon="org" testId="found-card">
              <form
                className="grid gap-3"
                data-testid="found-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  submit();
                }}
              >
                <Field label="Kind">
                  <Select aria-label="Kind" value={chosenKind} onChange={(e) => setKind(e.target.value)}>
                    {kinds.map((k) => (
                      <option key={k} value={k}>
                        {k.replace("_", " ")}
                      </option>
                    ))}
                  </Select>
                </Field>
                <Field label="Name">
                  <Input aria-label="Name" value={name} onChange={(e) => setName(e.target.value)} maxLength={40} />
                </Field>
                <fieldset className="m-0 grid gap-1 border-0 p-0">
                  <legend className="mb-1 text-sm font-bold">First workplace</legend>
                  <label className="flex min-h-touch items-center gap-2">
                    <input type="radio" name="wp" checked={wpKind === null} onChange={() => setWpKind(null)} />
                    <span>none yet</span>
                  </label>
                  {WORKPLACE_KINDS.map((k) => {
                    const s = slots[k];
                    const scarcity = s?.total == null ? "unlimited" : `${s.free ?? 0} of ${s.total} slots free`;
                    const none = s?.total != null && (s.free ?? 0) === 0;
                    return (
                      <label key={k} className={`flex min-h-touch items-center gap-2 ${none ? "text-muted" : ""}`}>
                        <input type="radio" name="wp" value={k} checked={wpKind === k} disabled={none} onChange={() => setWpKind(k)} />
                        <span>
                          {k.replace("_", " ")} <span className="text-muted text-sm">{scarcity}</span>
                        </span>
                      </label>
                    );
                  })}
                </fieldset>
                <Tile testId="found-cost" className="text-sm">
                  Costs {c.money ? <b className={canPayMoney ? "" : "text-crit"}>{credits(v.founding.money)} cr</b> : "nothing in money"}
                  {wpKind ? (
                    <>
                      {" "}
                      and <b className={canPayMaterials ? "" : "text-crit"}>{v.founding.materials} Materials</b> from your {t("pantry").toLowerCase()} (you hold {materialsHeld})
                    </>
                  ) : null}
                  . {c.money ? `Balance after: ${credits(h.household.balance - v.founding.money)} cr.` : ""} You become the owner of every share and the manager.
                </Tile>
                <ButtonRow>
                  <Button type="submit" variant="danger" disabled={found.isPending} disabledReason={cannotFound}>
                    Found it
                  </Button>
                  {error ? (
                    <span className="text-crit text-sm" role="alert">
                      {error}
                    </span>
                  ) : null}
                </ButtonRow>
              </form>
            </Card>
          ) : null}
        </Two>

        <Card title="Every organization" icon="org" aside={<span className="text-muted text-sm font-normal">{v.orgs.length}</span>}>
          <table className="w-full border-collapse text-[15px]" data-testid="all-orgs">
            <thead>
              <tr>
                <th className={TH}>Name</th>
                <th className={`${TH} hidden md:table-cell`}>Kind</th>
                <th className={`${TH} hidden md:table-cell`}>Workplaces</th>
                <th className={TH_NUM}>Staff</th>
                {c.money ? <th className={TH_NUM}>Book value</th> : null}
              </tr>
            </thead>
            <tbody>
              {v.orgs.map((o) => {
                const workplaces = o.workplaces.map((w) => w.kind.replace("_", " ")).join(", ") || "none";
                return (
                  <tr key={o.id} className="hover:bg-surface-2">
                    <td className={TD}>
                      <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(o.id) }}>
                        {o.name}
                      </Link>
                      {o.payment_missed ? (
                        <Pill tone="crit" className="ml-2">
                          missed a payday
                        </Pill>
                      ) : null}
                      <span className="text-muted block text-sm md:hidden">
                        {o.kind.replace("_", " ")} · {workplaces}
                      </span>
                    </td>
                    <td className={`${TD} hidden md:table-cell`}>{o.kind.replace("_", " ")}</td>
                    <td className={`${TD} hidden md:table-cell`}>{workplaces}</td>
                    <td className={TD_NUM}>{o.employees}</td>
                    {c.money ? <td className={TD_NUM}>{credits(o.book_value)}</td> : null}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </Card>

        <FooterStrip>
          <span>{h.society.population} citizens</span>
          <span>{h.society.active_humans} people</span>
          <span>{h.society.unemployed} without work</span>
          {h.society.price_index != null ? (
            <span>
              {t("society_stat").toLowerCase()} {h.society.price_index.toFixed(2)}
            </span>
          ) : null}
        </FooterStrip>
      </Stack>
    </div>
  );
}
