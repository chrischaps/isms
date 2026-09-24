// TypeSafe's Decisions API, as OpenRouter serves it (SJ.1): one POST with a
// `state`, a `model` and a map of typed questions; every question is answered
// against the state in one pass, with a probability on each answer. The call
// goes through the harness `Transport`, so a recording holds the exchange and
// a fixture can stand in for it. No SDK: the request shape is TypeSafe's own,
// and a chat-completions client cannot reach the model.

import { z } from "zod";
import type { Transport } from "../../api/transport.ts";

export type ChoiceQuestion = {
  type: "choice";
  /** Who is deciding and what turns on it; the persona lives here, never in the state. */
  instructions: string;
  /** Option key -> one sentence Jev reads as that option. */
  criteria: Record<string, string>;
};
export type Question = ChoiceQuestion;

export type DecisionRequest = { model: string; state: unknown; questions: Record<string, Question> };

/** An answer as Jev gives it; the fields depend on the question's type, so none is required. */
const AnswerSchema = z.looseObject({
  type: z.string().optional(),
  choice: z.string().optional(),
  probabilities: z.record(z.string(), z.number()).optional(),
  confidence: z.number().optional(),
  noul: z.number().optional(),
  score: z.number().optional(),
});
export type Answer = z.infer<typeof AnswerSchema>;

const ResponseSchema = z.looseObject({
  model: z.string().optional(),
  answers: z.record(z.string(), AnswerSchema),
  usage: z
    .looseObject({ input_tokens: z.number(), output_tokens: z.number().default(0), cost: z.number().optional() })
    .optional(),
});
export type DecisionResponse = z.infer<typeof ResponseSchema>;

export class JevError extends Error {
  readonly status: number;
  /** A key or credit problem: no retry fixes it, the brain stops asking. */
  readonly fatal: boolean;
  constructor(status: number, message: string, fatal = false) {
    super(message);
    this.name = "JevError";
    this.status = status;
    this.fatal = fatal;
  }
}

export type DecideOpts = {
  baseUrl: string;
  key: string;
  transport: Transport;
  /** Injected in tests. */
  sleep?: (ms: number) => Promise<void>;
};

const RETRY_AFTER_MS = 1000;

/** One decision call. A 429 or an overload is retried once after a second; anything else is an error. */
export async function decide(req: DecisionRequest, opts: DecideOpts): Promise<DecisionResponse> {
  const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)));
  const send = () =>
    opts.transport(opts.baseUrl, {
      method: "POST",
      headers: { authorization: `Bearer ${opts.key}`, "content-type": "application/json" },
      body: JSON.stringify(req),
    });
  let res = await send();
  // A rate limit, an overload, or a gateway between us and the model (OpenRouter answered 520 once in 1,162 turns): one retry.
  if (res.status === 429 || res.status === 529 || (res.status >= 500 && res.status !== 500)) {
    await sleep(RETRY_AFTER_MS);
    res = await send();
  }
  const text = await res.text();
  if (!res.ok) {
    const fatal = res.status === 401 || res.status === 402 || res.status === 403;
    throw new JevError(res.status, `${res.status} from the decisions API: ${text.slice(0, 300)}`, fatal);
  }
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new JevError(res.status, `the decisions API did not answer with JSON: ${text.slice(0, 120)}`);
  }
  const parsed = ResponseSchema.safeParse(json);
  if (!parsed.success) {
    throw new JevError(res.status, `unexpected answer shape: ${parsed.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`).join("; ")}`);
  }
  return parsed.data;
}
