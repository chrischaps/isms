// Names for the numbers in payloads (orgs, citizens, workplaces), from the
// two public directories every screen may read: the org list and the
// citizen list. A player never sees "org #15"; an id shows only as a last
// resort, spelled out, when the directory has no entry for it.

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, unwrap } from "../api/client";
import { civicKeys } from "../api/civic";
import { marketKeys } from "../api/market";

type OrgLike = { id: number; name: string; workplaces: { id: number; kind: string }[] };
type CitizenLike = { id: number; handle: string };

export type Names = {
  org: (id: number) => string;
  citizen: (id: number) => string;
  /** A `Party` payload: a citizen, an org, or the society itself. */
  party: (p: unknown) => string;
  /** "farm at Greenfield", for a workplace id seen outside its org's page. */
  workplace: (id: number) => string;
  /** "farm", or "farm 2" when its org has several: for use beside the org's name. */
  workplaceTitle: (id: number) => string;
  /** An engine message in plain words: "c41 already works at w19" names both. */
  inText: (text: string) => string;
};

/** "you" takes a plural verb where the engine wrote one for a citizen's id. */
const VERBS: Record<string, string> = { works: "work", holds: "hold", has: "have", does: "do", is: "are", owns: "own" };

export function kindName(kind: string): string {
  return kind.replaceAll("_", " ");
}

/** Each workplace by its kind; a number is added only when an org has several of one kind. */
export function workplaceTitles(workplaces: { id: number; kind: string }[]): Map<number, string> {
  const sorted = workplaces.slice().sort((a, b) => a.id - b.id);
  const titles = new Map<number, string>();
  for (const w of sorted) {
    const same = sorted.filter((x) => x.kind === w.kind);
    titles.set(w.id, same.length > 1 ? `${kindName(w.kind)} ${same.indexOf(w) + 1}` : kindName(w.kind));
  }
  return titles;
}

export function buildNames(orgs: OrgLike[], citizens: CitizenLike[], me?: number): Names {
  const orgNames = new Map(orgs.map((o) => [o.id, o.name]));
  const handles = new Map(citizens.map((c) => [c.id, c.handle]));
  const places = new Map<number, string>();
  const titles = new Map<number, string>();
  for (const o of orgs) {
    for (const [wid, title] of workplaceTitles(o.workplaces)) {
      titles.set(wid, title);
      places.set(wid, `${title} at ${o.name}`);
    }
  }

  const org = (id: number) => orgNames.get(id) ?? `organization no. ${id}`;
  const citizen = (id: number) => (id === me ? "you" : (handles.get(id) ?? `citizen no. ${id}`));
  const workplace = (id: number) => places.get(id) ?? `workplace no. ${id}`;
  // The engine prints ids as a letter and a number (isms-core ids.rs): c41, o15, w19, d7, k441, f30, r12, s3.
  const token = (letter: string, n: number): string => {
    switch (letter) {
      case "c":
        return citizen(n);
      case "o":
        return org(n);
      case "w":
        return `the ${workplace(n)}`;
      case "d":
        return `dwelling no. ${n}`;
      case "s":
        return `slot ${n}`;
      case "k":
        return "that contract";
      case "f":
        return "that offer";
      default:
        return "that order";
    }
  };
  const NOUNS = "citizen|org|workplace|dwelling|contract|offer|order|slot";
  // A handle or an org's name keeps its own spelling even at the start of a sentence.
  const startsWithName = (text: string) => {
    const m = /^(?:(?:Citizen|Org)\()?([co])(\d+)\b/.exec(text);
    return m != null && !(m[1] === "c" && Number(m[2]) === me);
  };
  const inText = (text: string) => {
    const out = text
      // `{:?}` of a Party: Citizen(c41), Org(o15).
      .replace(/\b(?:Citizen|Org)\(([co]\d+)\)/g, "$1")
      // "no workplace w19": the id names nothing, so say so.
      .replace(new RegExp(`\\bno (${NOUNS}) [cowdkfrs]\\d+\\b`, "g"), (_m, noun: string) => `there is no such ${noun === "org" ? "organization" : noun}`)
      // "contract k441 is not ...": the token alone carries the noun.
      .replace(new RegExp(`\\b(?:${NOUNS}) ([cowdkfrs]\\d+)\\b`, "g"), "$1")
      .replace(/\b([cowdkfrs])(\d+)\b/g, (_m, letter: string, n: string) => token(letter, Number(n)))
      .replace(/\byou (already )?(works|holds|has|does|is|owns)\b/g, (_m, already: string | undefined, verb: string) => `you ${already ?? ""}${VERBS[verb]}`)
      .replace(/\bcycles\b/g, "days")
      .replace(/\bcycle\b/g, "day")
      .replace(/\bticks\b/g, "hours")
      .replace(/\btick\b/g, "hour");
    return startsWithName(text) ? out : out.replace(/^./, (ch) => ch.toUpperCase());
  };
  return {
    inText,
    org,
    citizen,
    party: (p) => {
      if (p === "society") return "the society";
      const q = (p ?? {}) as Record<string, unknown>;
      if (typeof q.citizen === "number") return citizen(q.citizen);
      if (typeof q.org === "number") return org(q.org);
      return "someone";
    },
    workplace,
    workplaceTitle: (id) => titles.get(id) ?? `workplace no. ${id}`,
  };
}

/** `enabled: false` until the viewer is a citizen (both directories are members-only). */
export function useNames(id: number, me?: number, enabled = true): Names {
  // The same cache entries as `useOrgs` / `useCitizens`: whole responses, narrowed on read.
  const orgs = useQuery({
    queryKey: marketKeys.orgs(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/orgs", { params: { path: { id } } })),
    enabled,
  });
  const citizens = useQuery({
    queryKey: civicKeys.citizens(id),
    queryFn: async () => unwrap(await api.GET("/s/{id}/citizens", { params: { path: { id } } })),
    enabled,
  });
  return useMemo(() => buildNames(orgs.data?.orgs ?? [], citizens.data?.citizens ?? [], me), [orgs.data, citizens.data, me]);
}
