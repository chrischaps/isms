// Personas are data: a markdown file with frontmatter (who, which brain, goals,
// temperament, risk, optional model overrides) and a body the LLM brain reads
// as "who you are". They state goals, never a script; the scripted brain is a
// separate module keyed by the slug.

import { readFileSync } from "node:fs";
import matter from "gray-matter";
import { z } from "zod";
import type { Persona } from "../brain/brain.ts";

const Frontmatter = z.object({
  name: z.string(),
  slug: z.string().regex(/^[a-z0-9-]{2,20}$/),
  brain: z.enum(["scripted", "llm"]),
  goals: z.array(z.string()).min(1),
  temperament: z.string(),
  risk: z.enum(["low", "medium", "high"]),
  model: z.object({ turn: z.string().optional(), cycle: z.string().optional() }).default({}),
  max_actions_per_turn: z.number().int().min(1).nullable().default(null),
});

export function parsePersona(text: string): Persona {
  const { data, content } = matter(text);
  const fm = Frontmatter.parse(data);
  return { ...fm, body: content.trim() };
}

export function loadPersona(path: string): Persona {
  return parsePersona(readFileSync(path, "utf8"));
}

/** The persona as the model reads it: frontmatter facts in prose, then the body. */
export function personaText(p: Persona): string {
  return [
    `You are ${p.name}.`,
    `Temperament: ${p.temperament}. Appetite for risk: ${p.risk}.`,
    "Your goals, in order:",
    ...p.goals.map((g, i) => `${i + 1}. ${g}`),
    "",
    p.body,
  ].join("\n");
}
