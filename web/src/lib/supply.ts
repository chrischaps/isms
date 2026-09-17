// Where a good comes from and where it goes (GDD 5, the production chain),
// read off the preset's recipes and the public org directory: which kind of
// workplace makes it, from what, who runs one, what they made today and what
// they hold. It answers "nobody is selling Materials: will anybody ever?".

export type Recipe = { workplace_kind: string; produces: string; consumes: Record<string, number>; base_rate: number };

export type OrgLike = {
  id: number;
  name: string;
  inventory: Record<string, number>;
  workplaces: { id: number; kind: string; cycle_output: number; workers: unknown[] }[];
};

export type Producer = {
  org: number;
  name: string;
  workplaces: number;
  workers: number;
  /** Units made so far today, over the org's workplaces of this kind. */
  madeToday: number;
  /** Units of the good the org holds now (what it could put on the market). */
  stock: number;
  /** For each input the recipe uses up: how many the org holds. Zero of any means it cannot run. */
  inputs: Record<string, number>;
};

export type Supply = {
  /** The recipes that make the good; empty when nothing in this society does. */
  madeBy: Recipe[];
  producers: Producer[];
  /** The recipes that use the good up. */
  usedBy: Recipe[];
};

export function supplyOf(good: string, recipes: Recipe[], orgs: OrgLike[]): Supply {
  const madeBy = recipes.filter((r) => r.produces === good);
  const kinds = new Map(madeBy.map((r) => [r.workplace_kind, r]));
  const producers = orgs
    .flatMap((o): Producer[] => {
      const mine = o.workplaces.filter((w) => kinds.has(w.kind));
      if (mine.length === 0) return [];
      const inputs: Record<string, number> = {};
      for (const w of mine) for (const g of Object.keys(kinds.get(w.kind)!.consumes)) inputs[g] = o.inventory[g] ?? 0;
      return [
        {
          org: o.id,
          name: o.name,
          workplaces: mine.length,
          workers: mine.reduce((n, w) => n + w.workers.length, 0),
          madeToday: mine.reduce((n, w) => n + w.cycle_output, 0),
          stock: o.inventory[good] ?? 0,
          inputs,
        },
      ];
    })
    .sort((a, b) => b.stock - a.stock || b.madeToday - a.madeToday || a.name.localeCompare(b.name));
  return { madeBy, producers, usedBy: recipes.filter((r) => (r.consumes[good] ?? 0) > 0) };
}

/** "1 ore" / "2 materials and 1 ore" / "nothing but labor". */
export function inputsText(consumes: Record<string, number>): string {
  const parts = Object.entries(consumes).map(([g, n]) => `${n} ${g}`);
  if (parts.length === 0) return "nothing but labor";
  return parts.length === 1 ? parts[0]! : `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]!}`;
}

/** Why a producer is or is not likely to sell soon, in a few words. */
export function producerNote(p: Producer): string {
  const short = Object.entries(p.inputs)
    .filter(([, n]) => n === 0)
    .map(([g]) => g);
  if (p.workers === 0) return "nobody works there";
  if (short.length > 0) return `out of ${short.join(" and ")}, so it cannot run`;
  if (p.madeToday === 0) return "nothing made yet today";
  return "running";
}
