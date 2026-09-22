// Where a good comes from (lib/supply), on its market page: the recipe, the
// orgs that run that kind of workplace with what they made today and what
// they hold, the goods upstream of it, and what uses it up. An empty ask
// side reads very differently when nobody makes the good at all.

import { Link } from "@tanstack/react-router";
import { kindName } from "../lib/names";
import { inputsText, producerNote, supplyOf, type Recipe } from "../lib/supply";
import { TD, TD_NUM, TH, TH_NUM } from "./Ledger";

type OrgLike = Parameters<typeof supplyOf>[2][number];

const article = (word: string) => (/^[aeiou]/i.test(word) ? "an" : "a");
const places = (n: number, kind: string) => `${n} ${kindName(kind)}${n === 1 ? "" : "s"}`;

export function Provenance({
  id,
  good,
  recipes,
  orgs,
  foundingMaterials,
  bare,
}: {
  id: number;
  good: string;
  recipes: Recipe[];
  orgs: OrgLike[];
  /** Materials a new workplace costs, shown on the Materials page as one more use. */
  foundingMaterials?: number;
  /** The card that holds it already carries the title. */
  bare?: boolean;
}) {
  const s = supplyOf(good, recipes, orgs);
  const upstream = [...new Set(s.madeBy.flatMap((r) => Object.keys(r.consumes)))];
  const goodLink = (g: string) => (
    <Link to="/s/$id/market/$instrument" params={{ id: String(id), instrument: g }}>
      {g}
    </Link>
  );
  return (
    <section className="text-[15px]" data-testid="provenance">
      {bare ? null : <h4 className="text-muted text-xs font-bold tracking-caps uppercase">Where it comes from</h4>}
      {s.madeBy.length === 0 ? (
        <p className="mt-1">Nothing in this society makes {good}.</p>
      ) : (
        s.madeBy.map((r) => (
          <p key={r.workplace_kind} className="mt-1">
            Made in {article(kindName(r.workplace_kind))} {kindName(r.workplace_kind)}, from {inputsText(r.consumes)} a unit.
          </p>
        ))
      )}

      {s.madeBy.length > 0 && s.producers.length === 0 ? (
        <p className="text-attn mt-2">
          No {s.madeBy.map((r) => kindName(r.workplace_kind)).join(" or ")} exists yet, so none will reach this book until someone founds one.
        </p>
      ) : null}

      {s.producers.length > 0 ? (
        <table className="mt-2 w-full border-collapse" data-testid="producers">
          <thead>
            <tr>
              <th className={TH}>Producer</th>
              <th className={TH_NUM}>Workers</th>
              <th className={TH_NUM}>Made today</th>
              <th className={TH_NUM}>Holds</th>
            </tr>
          </thead>
          <tbody>
            {s.producers.map((p) => (
              <tr key={p.org} className="hover:bg-surface-2">
                <td className={TD}>
                  <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(p.org) }}>
                    {p.name}
                  </Link>
                  <span className="text-muted block text-xs">
                    {places(p.workplaces, s.madeBy[0]!.workplace_kind)} · {producerNote(p)}
                  </span>
                </td>
                <td className={TD_NUM}>{p.workers}</td>
                <td className={TD_NUM}>{p.madeToday.toFixed(0)}</td>
                <td className={TD_NUM}>{p.stock}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      {s.producers.length > 0 ? (
        <p className="text-muted mt-2 text-sm">
          What a producer holds reaches this book only when its manager posts an ask; it may also keep it, use it, or sell it directly.
        </p>
      ) : null}

      {upstream.length > 0 ? (
        <p className="mt-2">
          Upstream:{" "}
          {upstream.map((g, i) => (
            <span key={g}>
              {i > 0 ? ", " : ""}
              {goodLink(g)}
            </span>
          ))}
          . A producer short of {upstream.length === 1 ? "it" : "any of them"} stops.
        </p>
      ) : null}

      {s.usedBy.length > 0 || foundingMaterials ? (
        <p className="text-muted mt-2">
          Used up by:{" "}
          {[
            ...s.usedBy.map((r) => `${article(kindName(r.workplace_kind))} ${kindName(r.workplace_kind)} (${r.consumes[good]} a unit of ${r.produces})`),
            ...(foundingMaterials ? [`founding a workplace (${foundingMaterials})`] : []),
          ].join(", ")}
          .
        </p>
      ) : null}
    </section>
  );
}
