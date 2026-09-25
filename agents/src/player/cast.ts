// Who plays: the persona list for a run, cycled to the number of players. A
// preset may have its own list (S2.10) and a cast may be named outright (SJ.3:
// `--cast freeport-town`, the sixteen-player town); Freeport's list is the
// fallback. Kept apart from main.ts so a test can load a cast without running one.

import { readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Persona } from "../brain/brain.ts";
import type { Config } from "../config.ts";
import { loadPersona } from "./persona.ts";

export const AGENTS_DIR = resolve(import.meta.dirname, "..", "..");
export const PERSONAS_DIR = join(AGENTS_DIR, "personas");

/** The persona list for a preset: a named cast in `run.personas_for` (SJ.3), else the preset's own, else `run.personas` (Freeport's). */
export function personaSlugs(cfg: Config, preset: string, cast?: string): string[] {
  if (cast) {
    const list = cfg.run.personas_for[cast];
    if (!list) throw new Error(`no cast named ${cast} in run.personas_for (${Object.keys(cfg.run.personas_for).join(", ")})`);
    return list;
  }
  return cfg.run.personas_for[preset] ?? cfg.run.personas;
}

export function personasFor(cfg: Config, preset = "freeport", cast?: string): Persona[] {
  const available = new Map(readdirSync(PERSONAS_DIR).filter((f) => f.endsWith(".md")).map((f) => [f.replace(/\.md$/, ""), join(PERSONAS_DIR, f)]));
  const slugs = personaSlugs(cfg, preset, cast);
  const out: Persona[] = [];
  for (let i = 0; i < cfg.run.players; i++) {
    const slug = slugs[i % slugs.length]!;
    const file = available.get(slug);
    if (!file) throw new Error(`no persona file for ${slug} in ${PERSONAS_DIR}`);
    out.push(loadPersona(file));
  }
  return out;
}
