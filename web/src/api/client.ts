// The one API client (TDD 11, D9): typed from /openapi.json by
// openapi-typescript (`make api-types`), spoken through openapi-fetch. Every
// state-changing browser request carries the CSRF header (TDD 10.2).

import createClient, { type Middleware } from "openapi-fetch";
import type { components, paths } from "./schema";

export type Schemas = components["schemas"];
export type Clock = Schemas["Clock"];
export type Problem = Schemas["Problem"];
export type SocietySummary = Schemas["SocietySummary"];
export type CapabilitiesView = Schemas["CapabilitiesView"];
export type Lexicon = Schemas["Lexicon"];
export type HomeView = Schemas["HomeView"];
export type Me = Schemas["Me"];
export type Welcome = Schemas["Welcome"];
export type NoticeBoardView = Schemas["NoticeBoardView"];
export type OfferView = Schemas["OfferView"];
export type Committed = Schemas["Committed"];
/** Engine-shaped payloads are documented as `Object`; here they are open records. */
export type EventRef = Omit<Schemas["EventRef"], "payload"> & { payload: Record<string, unknown> };
/** One `/s/{id}/stream` frame (a WebSocket type, so not in the OpenAPI paths). */
export type StreamFrame = { clock: Clock; events: EventRef[]; lagged?: number | null };

const csrf: Middleware = {
  onRequest({ request }) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      request.headers.set("X-Requested-With", "isms");
    }
    return request;
  },
};

export const api = createClient<paths>({
  baseUrl: import.meta.env.VITE_API_BASE ?? "",
  credentials: "include",
});
api.use(csrf);

/** A failed call, with the server's problem document when there is one. */
export class ApiError extends Error {
  status: number;
  problem?: Problem;
  constructor(status: number, problem?: Problem) {
    super(problem?.detail ?? problem?.title ?? `HTTP ${status}`);
    this.status = status;
    this.problem = problem;
  }
}

/** Unwrap an openapi-fetch result into data or a thrown ApiError. */
export function unwrap<T>(r: {
  data?: T;
  error?: unknown;
  response: Response;
}): T {
  if (r.error !== undefined || !r.response.ok) {
    throw new ApiError(r.response.status, r.error as Problem | undefined);
  }
  return r.data as T;
}

/** Cents on the wire, credits on screen (TDD 10.4). */
export function credits(cents: number, fractionDigits = 2): string {
  return (cents / 100).toLocaleString(undefined, {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  });
}
