// The one API client (TDD 10.2): typed from /openapi.json like the web's, spoken
// through openapi-fetch. A player holds an API key (Bearer); only the bootstrap
// holds a browser session, and then it sends the CSRF header like a browser.

import createClient, { type Middleware } from "openapi-fetch";
import type { components, paths } from "./schema.d.ts";
import { liveTransport, type Transport } from "./transport.ts";

export type Schemas = components["schemas"];
export type Clock = Schemas["Clock"];
export type Problem = Schemas["Problem"];
export type HomeView = Schemas["HomeView"];
export type Committed = Schemas["Committed"];
export type EventRef = Omit<Schemas["EventRef"], "payload"> & { payload: Record<string, unknown> };
/** One `/s/{id}/stream` frame (a WebSocket type, so not in the OpenAPI paths). */
export type StreamFrame = { clock: Clock; events: EventRef[]; lagged?: number | null };

export type Auth = { key: string } | { session: string };

export type Client = ReturnType<typeof createClient<paths>>;

export function makeClient(opts: { baseUrl: string; auth: Auth; transport?: Transport }): Client {
  const transport = opts.transport ?? liveTransport;
  const client = createClient<paths>({
    baseUrl: opts.baseUrl,
    fetch: (req) => transport(req),
  });
  const auth: Middleware = {
    onRequest({ request }) {
      if ("key" in opts.auth) {
        request.headers.set("authorization", `Bearer ${opts.auth.key}`);
      } else {
        request.headers.set("cookie", `isms_session=${opts.auth.session}`);
        if (request.method !== "GET" && request.method !== "HEAD") {
          request.headers.set("x-requested-with", "isms");
        }
      }
      return request;
    },
  };
  client.use(auth);
  return client;
}

/** A failed call, carrying the server's problem document verbatim. */
export class ApiError extends Error {
  status: number;
  problem: Problem | undefined;
  constructor(status: number, problem?: Problem) {
    super(problem?.detail ?? problem?.title ?? `HTTP ${status}`);
    this.status = status;
    this.problem = problem;
  }
  /** The engine's RejectCode when the engine refused, else an HTTP status word. */
  get code(): string {
    return this.problem?.code ?? `HTTP_${this.status}`;
  }
}

/** Unwrap an openapi-fetch result into data or a thrown ApiError. */
export function unwrap<T>(r: { data?: T; error?: unknown; response: Response }): T {
  if (r.error !== undefined || !r.response.ok) {
    throw new ApiError(r.response.status, r.error as Problem | undefined);
  }
  return r.data as T;
}
