// The questions a Jev turn asks (SJ.1): one `choice` per slot, the persona in
// the instructions, the slot's candidates as the options. The persona's words
// live here and not in the state on purpose: TypeSafe says text in the state
// can steer an answer, and a headline should not vote.

import type { Persona } from "../brain.ts";
import { personaText } from "../../player/persona.ts";
import type { ChoiceQuestion } from "./client.ts";
import type { Slot } from "./candidates.ts";

/** `context` is the hour's one sentence about where the citizen stands (SJ.2); it rides with the persona, not in the state. */
export function buildQuestions(persona: Persona, slots: Slot[], context: string | null = null): Record<string, ChoiceQuestion> {
  const who = personaText(persona) + (context ? `\n\n${context}` : "");
  const out: Record<string, ChoiceQuestion> = {};
  for (const slot of slots) {
    out[slot.key] = {
      type: "choice",
      instructions: `${who}\n\nDecide only this: ${slot.ask} Pick the one option this person would take this hour.`,
      criteria: Object.fromEntries(slot.candidates.map((c) => [c.option, c.describe])),
    };
  }
  return out;
}
