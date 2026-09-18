// The one thing the harness does outside the public API: mint a browser session
// for a synthetic account with `isms-server session`, exactly as the e2e scripts
// do. The token is used once, to join and to create the player's API key, and is
// never written down.

import { execFile } from "node:child_process";
import { promisify } from "node:util";

const run = promisify(execFile);

export async function sessionToken(serverBin: string, email: string, databaseUrl: string | undefined): Promise<string> {
  const env = { ...process.env };
  if (databaseUrl) env.DATABASE_URL = databaseUrl;
  const { stdout } = await run(serverBin, ["session", "--email", email], { env });
  const token = stdout.trim().split(/\r?\n/).pop() ?? "";
  if (token === "") throw new Error(`isms-server session printed nothing for ${email}`);
  return token;
}
