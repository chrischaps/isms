// Organizations (GDD 6.1 firms, 9.2 office archetype; S1.11): the list of
// every org with what you hold in it, the public job board, and the
// found-a-firm flow with the slot picker showing scarcity and the cost
// preview against your balance and pantry. Kinds come from capabilities.

import { useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { ApiError, credits } from "../api/client";
import { useBoard, useCapabilities, useHome, useLexicon } from "../api/hooks";
import { useAcceptOffer } from "../api/society";
import { useFoundOrg, useOrgsView, type OrgView } from "../api/orgs";
import { jobLine } from "../lib/offers";

const WORKPLACE_KINDS = ["farm", "mine", "foundry", "mill", "workshop", "machine_shop", "builder"];

function holding(o: OrgView, me: number): string | null {
  const own = o.ownership as Record<string, unknown>;
  const shares = own.shares as { issued: number; holdings: Record<string, number> } | undefined;
  if (shares && o.my_shares > 0) {
    return `${o.my_shares} of ${shares.issued} shares (${((100 * o.my_shares) / shares.issued).toFixed(0)}%)`;
  }
  if (o.members.includes(me)) return "member";
  return null;
}

export function Orgs({ id }: { id: number }) {
  const { t } = useLexicon(id);
  const caps = useCapabilities(id);
  const home = useHome(id);
  const orgs = useOrgsView(id);
  const board = useBoard(id);
  const found = useFoundOrg(id);
  const accept = useAcceptOffer(id);
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<string | null>(null);
  const [wpKind, setWpKind] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (home.isPending || orgs.isPending || caps.isPending) return <p className="text-muted">Loading.</p>;
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
  if (home.error || orgs.error || caps.error) {
    return <p className="text-bad">Could not load: {String(home.error ?? orgs.error ?? caps.error)}</p>;
  }
  const h = home.data!;
  const v = orgs.data!;
  const c = caps.data!;
  const me = h.citizen.id;
  const names = new Map(v.orgs.map((o) => [o.id, o.name]));
  const kinds = c.org_kinds;
  const chosenKind = kind ?? kinds[0] ?? "firm";
  const slots = v.slots as Record<string, { total?: number | null; free?: number | null }>;
  const materialsHeld = (h.household.pantry as Record<string, number>).materials ?? 0;
  const canPayMoney = !c.money || h.household.balance >= v.founding.money;
  const canPayMaterials = wpKind === null || materialsHeld >= v.founding.materials;
  const slotFree = wpKind === null || slots[wpKind]?.total == null || (slots[wpKind]?.free ?? 0) > 0;
  const jobs = (board.data?.offers ?? []).map((o) => ({ o, line: jobLine(o) })).filter((j) => j.line !== null);
  const mine = v.orgs.filter((o) => o.i_manage || o.my_shares > 0 || o.members.includes(me));

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
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-2xl">Organizations</h2>
        <p className="text-muted text-sm">
          {v.orgs.length} in all{mine.length > 0 ? `, ${mine.length} yours` : ""}
        </p>
      </header>

      {mine.length > 0 ? (
        <section>
          <h3 className="text-lg">Yours</h3>
          <ul className="mt-2 flex flex-col gap-1 text-sm" data-testid="my-orgs">
            {mine.map((o) => (
              <li key={o.id} className="rule flex flex-wrap items-baseline justify-between gap-2 pt-1">
                <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(o.id) }} className="underline">
                  {o.name}
                </Link>
                <span className="text-muted">
                  {o.kind}
                  {o.i_manage ? " · you manage it" : ""}
                  {holding(o, me) ? ` · ${holding(o, me)}` : ""}
                </span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section className="grid gap-8 md:grid-cols-2">
        <div>
          <h3 className="text-lg">Job board</h3>
          <p className="text-muted mt-1 text-xs">Open {t("job").toLowerCase()}s from the notice board. Taking one sets nothing else; hours live on {t("work_screen")}.</p>
          {jobs.length === 0 ? (
            <p className="text-muted mt-2 text-sm">Nobody is hiring this tick.</p>
          ) : (
            <table className="mt-2 w-full text-sm" data-testid="job-board">
              <thead className="text-muted text-left text-xs uppercase tracking-wide">
                <tr>
                  <th className="py-1 font-normal">Firm</th>
                  <th className="py-1 font-normal">Pay</th>
                  <th className="py-1 font-normal">Terms</th>
                  <th className="py-1 font-normal" />
                </tr>
              </thead>
              <tbody>
                {jobs.map(({ o, line }) => (
                  <tr key={o.id} className="rule align-top">
                    <td className="py-1 pr-2">
                      <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(line!.org) }} className="underline">
                        {names.get(line!.org) ?? `org #${line!.org}`}
                      </Link>
                      <span className="text-muted block text-xs">workplace {line!.workplace}</span>
                    </td>
                    <td className="num py-1 pr-2 whitespace-nowrap">{line!.pay}</td>
                    <td className="py-1 pr-2 text-xs">
                      up to {line!.hours} h · {line!.term} · notice {line!.notice} · {line!.places} open
                    </td>
                    <td className="py-1 text-right">
                      <button
                        type="button"
                        disabled={accept.isPending}
                        className="border-line rounded-sm border px-2 py-0.5 text-xs"
                        onClick={() => accept.mutate(o.id, { onError: (e) => setError(e.message) })}
                      >
                        Take it
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {kinds.length > 0 ? (
          <form
            className="flex flex-col gap-3 text-sm"
            data-testid="found-form"
            onSubmit={(e) => {
              e.preventDefault();
              submit();
            }}
          >
            <h3 className="text-lg">Found one</h3>
            <label className="flex items-center gap-2">
              <span className="w-24">Kind</span>
              <select aria-label="Kind" className="border-line rounded-sm border px-1" value={chosenKind} onChange={(e) => setKind(e.target.value)}>
                {kinds.map((k) => (
                  <option key={k} value={k}>
                    {k.replace("_", " ")}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-2">
              <span className="w-24">Name</span>
              <input aria-label="Name" className="border-line w-56 rounded-sm border px-1" value={name} onChange={(e) => setName(e.target.value)} maxLength={40} />
            </label>
            <fieldset className="flex flex-col gap-1">
              <legend className="mb-1">First workplace</legend>
              <label className="flex items-center gap-2">
                <input type="radio" name="wp" checked={wpKind === null} onChange={() => setWpKind(null)} />
                <span>none yet</span>
              </label>
              {WORKPLACE_KINDS.map((k) => {
                const s = slots[k];
                const scarcity = s?.total == null ? "unlimited" : `${s.free ?? 0} of ${s.total} slots free`;
                const none = s?.total != null && (s.free ?? 0) === 0;
                return (
                  <label key={k} className={`flex items-center gap-2 ${none ? "text-muted" : ""}`}>
                    <input type="radio" name="wp" value={k} checked={wpKind === k} disabled={none} onChange={() => setWpKind(k)} />
                    <span>
                      {k.replace("_", " ")} <span className="text-muted text-xs">{scarcity}</span>
                    </span>
                  </label>
                );
              })}
            </fieldset>
            <div className="bg-paper-2 rounded-sm p-2 text-xs" data-testid="found-cost">
              Costs {c.money ? <span className={`num ${canPayMoney ? "" : "text-bad"}`}>{credits(v.founding.money)} cr</span> : "nothing in money"}
              {wpKind ? (
                <>
                  {" "}
                  and <span className={`num ${canPayMaterials ? "" : "text-bad"}`}>{v.founding.materials} Materials</span> from your {t("pantry").toLowerCase()} (you hold {materialsHeld})
                </>
              ) : null}
              . {c.money ? `Balance after: ${credits(h.household.balance - v.founding.money)} cr.` : ""} You become the owner of every share and the manager.
            </div>
            <div className="flex items-baseline gap-3">
              <button
                type="submit"
                disabled={found.isPending || name.trim().length === 0 || !canPayMoney || !canPayMaterials || !slotFree}
                className="bg-ink text-paper rounded-sm px-3 py-1 disabled:opacity-50"
              >
                Found it
              </button>
              {error ? (
                <span className="text-bad" role="alert">
                  {error}
                </span>
              ) : null}
            </div>
          </form>
        ) : null}
      </section>

      <section>
        <h3 className="text-lg">Every organization</h3>
        <table className="mt-2 w-full text-sm" data-testid="all-orgs">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Name</th>
              <th className="py-1 font-normal">Kind</th>
              <th className="py-1 font-normal">Workplaces</th>
              <th className="py-1 text-right font-normal">Staff</th>
              {c.money ? <th className="py-1 text-right font-normal">Book value</th> : null}
            </tr>
          </thead>
          <tbody>
            {v.orgs.map((o) => (
              <tr key={o.id} className="rule">
                <td className="py-1 pr-2">
                  <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(o.id) }} className="underline">
                    {o.name}
                  </Link>
                  {o.payment_missed ? <span className="text-bad ml-2 text-xs">missed a payday</span> : null}
                </td>
                <td className="py-1 pr-2">{o.kind.replace("_", " ")}</td>
                <td className="py-1 pr-2">{o.workplaces.map((w) => w.kind.replace("_", " ")).join(", ") || "none"}</td>
                <td className="num py-1 text-right">{o.employees}</td>
                {c.money ? <td className="num py-1 text-right">{credits(o.book_value)}</td> : null}
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
