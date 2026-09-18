// The one seam every HTTP call passes through: a fetch-shaped function. The
// live run uses the platform fetch; tests use fixtures; the recorder wraps
// fetch and writes each exchange; replay serves a recording in order. Both
// the game client and the Anthropic client take a Transport, so one recording
// holds a whole turn.

import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

export type FetchInput = Parameters<typeof fetch>[0];
export type Transport = (input: FetchInput, init?: RequestInit) => Promise<Response>;

export const liveTransport: Transport = (input, init) => fetch(input, init);

/** A recorded exchange (one JSONL line). Secrets are never written. */
export type Exchange = {
  i: number;
  target: "game" | "anthropic" | "other";
  method: string;
  url: string;
  reqBody: string | null;
  status: number;
  resBody: string;
};

function targetOf(url: string): Exchange["target"] {
  if (url.includes("api.anthropic.com")) return "anthropic";
  if (/^\/(s|societies|me|public|auth)(\/|$)/.test(new URL(url).pathname)) return "game";
  return "other";
}

async function toRequest(input: FetchInput, init?: RequestInit): Promise<Request> {
  return input instanceof Request && init === undefined ? input : new Request(input, init);
}

/** `METHOD /path` plus a query when present: the fixture and replay key. */
export function keyOf(method: string, url: string): string {
  const u = new URL(url);
  return `${method.toUpperCase()} ${u.pathname}${u.search}`;
}

/** Fixture file name for a key: `GET /s/1/home` -> `GET_s_1_home.json`. */
export function fixtureName(key: string): string {
  return key.replace(/[^A-Za-z0-9]+/g, "_").replace(/^_|_$/g, "") + ".json";
}

export function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

/** Serves `dir/<fixtureName(key)>` as `{status, body}`; a problem 404 when absent. */
export function fixtureTransport(
  dir: string,
  overrides: Record<string, { status: number; body: unknown }> = {},
): Transport {
  return async (input, init) => {
    const req = await toRequest(input, init);
    const key = keyOf(req.method, req.url);
    const o = overrides[key];
    if (o) return jsonResponse(o.status, o.body);
    const path = join(dir, fixtureName(key));
    if (!existsSync(path)) {
      return jsonResponse(404, {
        type: "about:blank",
        title: "no fixture",
        status: 404,
        detail: `no fixture for ${key}`,
      });
    }
    const { status, body } = JSON.parse(readFileSync(path, "utf8")) as { status: number; body: unknown };
    return jsonResponse(status, body);
  };
}

/** Wraps a transport and appends every exchange to `file`. */
export function recordingTransport(inner: Transport, file: string): Transport {
  mkdirSync(dirname(file), { recursive: true });
  let i = 0;
  return async (input, init) => {
    const req = await toRequest(input, init);
    const reqBody = req.method === "GET" || req.method === "HEAD" ? null : await req.clone().text();
    const res = await inner(req);
    const resBody = await res.clone().text();
    const line: Exchange = {
      i: i++,
      target: targetOf(req.url),
      method: req.method,
      url: req.url,
      reqBody,
      status: res.status,
      resBody,
    };
    appendFileSync(file, JSON.stringify(line) + "\n");
    return res;
  };
}

/** Request bodies are compared with sorted keys, so key order and whitespace never fail a replay. */
export function canonical(body: string | null): string | null {
  if (body === null) return null;
  try {
    return JSON.stringify(sortKeys(JSON.parse(body)));
  } catch {
    return body;
  }
}

function sortKeys(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(sortKeys);
  if (v && typeof v === "object") {
    const o = v as Record<string, unknown>;
    return Object.fromEntries(
      Object.keys(o)
        .sort()
        .map((k) => [k, sortKeys(o[k])]),
    );
  }
  return v;
}

/** Serves a recording in order; a request that differs from the next line is an error. */
export function replayTransport(lines: Exchange[]): Transport & { remaining(): number } {
  let at = 0;
  const t: Transport = async (input, init) => {
    const req = await toRequest(input, init);
    const next = lines[at];
    if (!next) throw new Error(`replay exhausted at request ${at}: ${req.method} ${req.url}`);
    const reqBody = req.method === "GET" || req.method === "HEAD" ? null : await req.clone().text();
    if (next.method !== req.method || next.url !== req.url || canonical(next.reqBody) !== canonical(reqBody)) {
      throw new Error(
        `replay mismatch at ${at}: expected ${next.method} ${next.url}\n  got ${req.method} ${req.url}\n  expected body ${next.reqBody}\n  got body ${reqBody}`,
      );
    }
    at += 1;
    return new Response(next.resBody, { status: next.status, headers: { "content-type": "application/json" } });
  };
  return Object.assign(t, { remaining: () => lines.length - at });
}

export function readRecording(file: string): Exchange[] {
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((l) => l.trim() !== "")
    .map((l) => JSON.parse(l) as Exchange);
}
