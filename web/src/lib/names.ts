// Names for the numbers in payloads (orgs, citizens, workplaces), from the
// two public directories every screen may read: the org list and the
// citizen list. A player never sees "org #15"; an id shows only as a last
// resort, spelled out, when the directory has no entry for it.

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
};

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
  return {
    org,
    citizen,
    party: (p) => {
      if (p === "society") return "the society";
      const q = (p ?? {}) as Record<string, unknown>;
      if (typeof q.citizen === "number") return citizen(q.citizen);
      if (typeof q.org === "number") return org(q.org);
      return "someone";
    },
    workplace: (id) => places.get(id) ?? `workplace no. ${id}`,
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
  return buildNames(orgs.data?.orgs ?? [], citizens.data?.citizens ?? [], me);
}
