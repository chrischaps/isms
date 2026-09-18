// How a turn ends. The rules ask every brain to finish with `end_turn`, stating
// what it meant to do this tick and anything it did not understand: the second
// field is the datum this whole harness exists to collect (ADR-0009).

import { z } from "zod";
import type { ToolDef } from "./context.ts";

export const EndTurnInput = z.object({
  intent: z.string().max(400).describe("One or two sentences: what you set out to do this hour, and whether it worked."),
  did_not_understand: z
    .array(z.string().max(300))
    .max(8)
    .describe(
      "Anything about the game, its words, a screen, an endpoint, or a refusal that you could not make sense of. Quote the text you saw. Empty when everything was clear.",
    ),
});
export type EndTurnInput = z.infer<typeof EndTurnInput>;

export const endTurn: ToolDef<EndTurnInput> = {
  name: "end_turn",
  kind: "end",
  description:
    "Finish this hour's turn. Call it last, every turn, even when you did nothing. Say what you intended and list anything you did not understand, quoting the exact words.",
  schema: EndTurnInput,
  run: async (input, ctx) => {
    ctx.turn.ended = { intent: input.intent, did_not_understand: input.did_not_understand };
    return { ok: true, data: { turn: "over" } };
  },
};
