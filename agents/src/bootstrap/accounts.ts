// Synthetic accounts (ADR-0009): `<persona>-<n>@agents.isms.test`, one citizen
// each, one API key each. Keys are persisted under the run so a stopped run can
// resume with the same players; session tokens are not.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { makeClient, unwrap } from "../api/client.ts";
import type { Transport } from "../api/transport.ts";
import { sessionToken } from "./server.ts";

export type PlayerAccount = {
  persona: string;
  n: number;
  email: string;
  handle: string;
  citizenId: number;
  key: string;
};

export const AGENT_DOMAIN = "agents.isms.test";

export function emailFor(persona: string, n: number): string {
  return `${persona}-${n}@${AGENT_DOMAIN}`.toLowerCase();
}

/** Handles are 2-24 of [A-Za-z0-9_-]; persona slugs already are. */
export function handleFor(persona: string, n: number): string {
  return `${persona}-${n}`.slice(0, 24);
}

export type BootstrapOpts = {
  baseUrl: string;
  society: number;
  serverBin: string;
  databaseUrl: string | undefined;
  runLabel: string;
  transport?: Transport;
  /** Injected in tests; the default shells out to `isms-server session`. */
  mintSession?: (email: string) => Promise<string>;
};

/** Session -> join (idempotent) -> API key, for one persona. */
export async function ensurePlayer(persona: string, n: number, opts: BootstrapOpts): Promise<PlayerAccount> {
  const email = emailFor(persona, n);
  const mint = opts.mintSession ?? ((e: string) => sessionToken(opts.serverBin, e, opts.databaseUrl));
  const session = await mint(email);
  const asBrowser = makeClient({ baseUrl: opts.baseUrl, auth: { session }, transport: opts.transport });
  const joined = unwrap(
    await asBrowser.POST("/societies/{id}/join", {
      params: { path: { id: opts.society } },
      body: { handle: handleFor(persona, n) },
    }),
  );
  const created = unwrap(
    await asBrowser.POST("/me/api-keys", {
      body: { label: `agents ${opts.runLabel}`, society_id: opts.society },
    }),
  );
  return { persona, n, email, handle: joined.handle, citizenId: joined.citizen_id, key: created.key };
}

/** Every player of the run, reusing `accounts.json` when it exists. */
export async function ensurePlayers(
  personas: string[],
  opts: BootstrapOpts,
  accountsFile: string,
): Promise<PlayerAccount[]> {
  if (existsSync(accountsFile)) {
    const saved = JSON.parse(readFileSync(accountsFile, "utf8")) as PlayerAccount[];
    if (saved.length === personas.length && saved.every((a, i) => a.persona === personas[i])) return saved;
  }
  const accounts: PlayerAccount[] = [];
  const counts = new Map<string, number>();
  for (const persona of personas) {
    const n = (counts.get(persona) ?? 0) + 1;
    counts.set(persona, n);
    accounts.push(await ensurePlayer(persona, n, opts));
  }
  mkdirSync(dirname(accountsFile), { recursive: true });
  writeFileSync(accountsFile, JSON.stringify(accounts, null, 2) + "\n");
  return accounts;
}
