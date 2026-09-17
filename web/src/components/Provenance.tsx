// Where a good comes from (lib/supply), on its market page: the recipe, the
// orgs that run that kind of workplace with what they made today and what
// they hold, the goods upstream of it, and what uses it up. An empty ask
// side reads very differently when nobody makes the good at all.

import { Link } from "@tanstack/react-router";
import { kindName } from "../lib/names";
import { inputsText, producerNote, supplyOf, type Recipe } from "../lib/supply";

type OrgLike = Parameters<typeof supplyOf>[2][number];

const article = (word: string) => (/^[aeiou]/i.test(word) ? "an" : "a");
const places = (n: number, kind: string) => `${n} ${kindName(kind)}${n === 1 ? "" : "s"}`;

export function Provenance({
  id,
  good,
  recipes,
  orgs,
  foundingMaterials,
}: {
  id: number;
  good: string;
  recipes: Recipe[];
  orgs: OrgLike[];
  /** Materials a new workplace costs, shown on the Materials page as one more use. */
  foundingMaterials?: number;
}) {
  const s = supplyOf(good, recipes, orgs);
  const upstream = [...new Set(s.madeBy.flatMap((r) => Object.keys(r.consumes)))];
  const goodLink = (g: string) => (
    <Link to="/s/$id/market/$instrument" params={{ id: String(id), instrument: g }} className="underline">
      {g}
    </Link>
  );
  return (
    <section className="text-sm" data-testid="provenance">
      <h4 className="text-muted text-xs uppercase tracking-wide">Where it comes from</h4>
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
        <p className="text-warn mt-2">
          No {s.madeBy.map((r) => kindName(r.workplace_kind)).join(" or ")} exists yet, so none will reach this book until someone founds one.
        </p>
      ) : null}

      {s.producers.length > 0 ? (
        <table className="mt-2 w-full" data-testid="producers">
          <thead className="text-muted text-left text-xs uppercase tracking-wide">
            <tr>
              <th className="py-1 font-normal">Producer</th>
              <th className="py-1 text-right font-normal">Workers</th>
              <th className="py-1 text-right font-normal">Made today</th>
              <th className="py-1 text-right font-normal">Holds</th>
            </tr>
          </thead>
          <tbody>
            {s.producers.map((p) => (
              <tr key={p.org} className="rule align-top">
                <td className="py-1 pr-2">
                  <Link to="/s/$id/orgs/$oid" params={{ id: String(id), oid: String(p.org) }} className="underline">
                    {p.name}
                  </Link>
                  <span className="text-muted block text-xs">
                    {places(p.workplaces, s.madeBy[0]!.workplace_kind)} · {producerNote(p)}
                  </span>
                </td>
                <td className="num py-1 text-right">{p.workers}</td>
                <td className="num py-1 text-right">{p.madeToday.toFixed(0)}</td>
                <td className="num py-1 text-right">{p.stock}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      {s.producers.length > 0 ? (
        <p className="text-muted mt-1 text-xs">
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
