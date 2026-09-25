// The command line: bootstrap the accounts, run the cohort, fold a run into a
// report, or record one player for the replay test.
//
//   pnpm run -- --run <name> [--preset freeport|commune] [--cast <name>] [--players N] [--brain scripted|llm|mixed|jev] [--model id] [--cycle-model id] [--max-usd n] [--record <player>] [--journal-questions]
//   pnpm bootstrap -- --run <name>
//   pnpm report <run>
//   pnpm rollover -- --run <name> [--wait <seconds>]   (S1.15: a closing statement, the archive, the next epoch)
//
// Environment: ISMS_URL, ISMS_SOCIETY, ISMS_SERVER_BIN, DATABASE_URL, ANTHROPIC_API_KEY, OPENROUTER_API_KEY.

import Anthropic from "@anthropic-ai/sdk";
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { ApiError, makeClient, unwrap, type Client } from "./api/client.ts";
import { liveTransport, recordingTransport, type Transport } from "./api/transport.ts";
import { ensurePlayers, type PlayerAccount } from "./bootstrap/accounts.ts";
import type { Brain, Persona } from "./brain/brain.ts";
import { JevBrain } from "./brain/jev/index.ts";
import { AnthropicBrain } from "./brain/llm/anthropic.ts";
import { ScriptedBrain } from "./brain/scripted/index.ts";
import { Budget } from "./budget/budget.ts";
import { runCohort } from "./cohort/cohort.ts";
import { loadConfig, readEnv, type Config } from "./config.ts";
import { Journal } from "./journal/journal.ts";
import { AGENTS_DIR, personasFor } from "./player/cast.ts";
import { Notes } from "./player/notes.ts";
import { Player } from "./player/player.ts";
import { buildReport, fold } from "./report/report.ts";
import { hasAssembly, readFacts, type SocietyFacts } from "./society.ts";
import { watchTicks, type TickSignal } from "./stream/ticks.ts";

export { AGENTS_DIR, PERSONAS_DIR, personaSlugs, personasFor } from "./player/cast.ts";
export const RUNS_DIR = join(AGENTS_DIR, "runs");
export const REPORTS_DIR = resolve(AGENTS_DIR, "..", "docs", "playtest", "runs");

/** What plays the persona (SJ.1): a run's `jev` puts the `[jev]` personas on Jev and scripts the rest. */
export function wantsBrain(persona: Persona, cfg: Config): Persona["brain"] {
  if (cfg.run.brain === "mixed") return persona.brain;
  if (cfg.run.brain === "jev") return cfg.jev.personas.includes(persona.slug) ? "jev" : "scripted";
  return cfg.run.brain;
}

export type JevWiring = { key: string | undefined; transport: Transport };

export function brainFor(persona: Persona, cfg: Config, mk: () => Anthropic, budget: Budget, player: string, handle: string, facts?: SocietyFacts, jev?: JevWiring): Brain {
  const wants = wantsBrain(persona, cfg);
  if (wants === "scripted") return new ScriptedBrain(persona, handle, facts);
  if (wants === "jev") {
    if (!jev?.key) throw new Error("OPENROUTER_API_KEY is not set and at least one player has a Jev brain");
    return new JevBrain({ persona, player, handle, cfg, budget, facts, key: jev.key, transport: jev.transport });
  }
  return new AnthropicBrain({ client: mk(), persona, player, cfg, budget, facts });
}

function log(line: string) {
  const at = new Date().toISOString().slice(11, 19);
  console.log(`${at} ${line}`);
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** What the epoch-end sequence left behind, for the report (S1.15). */
export type Rollover = {
  archived_epoch: number;
  reason: string;
  final_cycle: number;
  ended_at: string;
  closes_at: string;
  statements: number;
  statement_by: string | null;
  /** When the next epoch was first seen ticking; null when the wait ran out. */
  next_epoch_at: string | null;
  next_epoch: number | null;
};

/** After a run that ended with the epoch: the first player leaves a closing
 *  statement, then we wait for the window to close and the next epoch to
 *  tick on its own. Writes `rollover.json` beside the journals. */
export async function rollover(client: Client, sid: number, player: PlayerAccount, waitSeconds: number): Promise<Rollover> {
  const deadline = Date.now() + waitSeconds * 1000;
  const path = { params: { path: { id: sid } } };
  let archive = null;
  while (archive === null) {
    const v = unwrap(await client.GET("/s/{id}/archives", path));
    archive = v.archives.at(-1) ?? null;
    if (archive === null) {
      if (Date.now() > deadline) throw new Error(`no epoch archive within ${waitSeconds}s`);
      await sleep(1000);
    }
  }
  log(`archive: epoch ${archive.epoch} ended (${archive.reason}) after day ${archive.final_cycle}; statements close at ${archive.closes_at}`);
  let by: string | null = null;
  try {
    const written = unwrap(
      await client.PUT("/s/{id}/closing-statement", {
        ...path,
        body: { text: `${player.handle} played this epoch as the ${player.persona}. The ledgers are what they are.` },
      }),
    );
    by = player.handle;
    archive = written;
    log(`closing statement left by ${player.handle}; the archive holds ${written.closing_statements.length}`);
  } catch (e) {
    log(`closing statement refused: ${e instanceof ApiError ? `${e.code} ${e.message}` : String(e)}`);
  }
  let nextAt: string | null = null;
  let next: number | null = null;
  while (nextAt === null && Date.now() <= deadline) {
    const s = unwrap(await client.GET("/s/{id}/stats", path));
    if (s.clock.epoch > archive.epoch && s.clock.engine_tick >= 1) {
      nextAt = new Date().toISOString();
      next = s.clock.epoch;
      log(`epoch ${s.clock.epoch} is ticking (day ${s.clock.cycle}, hour ${s.clock.tick})`);
    } else await sleep(1000);
  }
  if (nextAt === null) log(`no rollover within ${waitSeconds}s`);
  return {
    archived_epoch: archive.epoch,
    reason: archive.reason,
    final_cycle: archive.final_cycle,
    ended_at: archive.ended_at,
    closes_at: archive.closes_at,
    statements: archive.closing_statements.length,
    statement_by: by,
    next_epoch_at: nextAt,
    next_epoch: next,
  };
}

async function main(argv: string[]) {
  const [cmd, ...rest] = argv;
  if (cmd === "report") {
    const run = rest[0];
    if (!run) throw new Error("usage: report <run>");
    const runDir = join(RUNS_DIR, run);
    const md = buildReport(fold(runDir, run));
    mkdirSync(REPORTS_DIR, { recursive: true });
    const file = join(REPORTS_DIR, `${run}.md`);
    writeFileSync(file, md);
    console.log(file);
    return;
  }
  if (cmd === "rollover") {
    const { values } = parseArgs({
      args: rest.filter((a) => a !== "--"),
      options: { run: { type: "string" }, wait: { type: "string", default: "180" } },
    });
    if (!values.run) throw new Error("usage: rollover --run <name> [--wait <seconds>]");
    const env = readEnv();
    const runDir = join(RUNS_DIR, values.run);
    const accountsFile = join(runDir, "accounts.json");
    if (!existsSync(accountsFile)) throw new Error(`${accountsFile} is missing: run the cohort first`);
    const accounts = JSON.parse(readFileSync(accountsFile, "utf8")) as PlayerAccount[];
    const first = accounts[0];
    if (!first) throw new Error("no players in accounts.json");
    const client = makeClient({ baseUrl: env.ismsUrl, auth: { key: first.key } });
    const r = await rollover(client, env.society, first, Number(values.wait));
    writeFileSync(join(runDir, "rollover.json"), JSON.stringify(r, null, 2) + "\n");
    // The report was written when the cohort stopped; write it again with the ending in it.
    const md = buildReport(fold(runDir, values.run));
    writeFileSync(join(runDir, "report.md"), md);
    mkdirSync(REPORTS_DIR, { recursive: true });
    writeFileSync(join(REPORTS_DIR, `${values.run}.md`), md);
    if (r.next_epoch_at === null) process.exitCode = 1;
    return;
  }
  if (cmd !== "run" && cmd !== "bootstrap" && cmd !== "record") throw new Error("usage: run | bootstrap | report | record | rollover");

  // pnpm hands a literal `--` through to the script.
  const { values } = parseArgs({
    args: rest.filter((a) => a !== "--"),
    options: {
      run: { type: "string" },
      config: { type: "string", default: join(AGENTS_DIR, "config.toml") },
      players: { type: "string" },
      brain: { type: "string" },
      model: { type: "string" },
      "cycle-model": { type: "string" },
      "cycle-provider": { type: "string" },
      "max-usd": { type: "string" },
      record: { type: "string" },
      // The preset the society was seeded from: picks the persona list (S2.10). The facts themselves are read from the server.
      preset: { type: "string", default: "freeport" },
      // A named cast from `run.personas_for` over the preset's list (SJ.3): `--cast freeport-town`.
      cast: { type: "string" },
      // Keep every Jev turn's request (state and questions) in the journal beside its decisions (E-6).
      "journal-questions": { type: "boolean" },
    },
  });
  const run = values.run ?? new Date().toISOString().slice(0, 16).replace(/[-:T]/g, "");
  const brainArg = values.brain as Config["run"]["brain"] | undefined;
  const cfg = loadConfig(values.config!, {
    players: cmd === "record" ? 1 : values.players ? Number(values.players) : undefined,
    brain: brainArg,
    turnModel: values.model,
    cycleModel: values["cycle-model"],
    cycleProvider: values["cycle-provider"] as Config["models"]["cycle_provider"] | undefined,
    maxUsd: values["max-usd"] ? Number(values["max-usd"]) : undefined,
    journalQuestions: values["journal-questions"] ? true : undefined,
  });
  const env = readEnv();
  const runDir = join(RUNS_DIR, run);
  mkdirSync(runDir, { recursive: true });
  const personas = personasFor(cfg, values.preset!, values.cast);
  const recording = cmd === "record" ? (values.record ?? `${personas[0]!.slug}-1`) : values.record;

  log(`run ${run}: preset ${values.preset}${values.cast ? `, cast ${values.cast}` : ""}, ${cfg.run.players} players, brain ${cfg.run.brain}, turn model ${cfg.models.turn}, cycle model ${cfg.models.cycle} via ${cfg.models.cycle_provider === "claude_code" ? "claude -p (subscription)" : "the API"}, society ${env.society} at ${env.ismsUrl}`);
  const accounts = await ensurePlayers(
    personas.map((p) => p.slug),
    { baseUrl: env.ismsUrl, society: env.society, serverBin: env.serverBin, databaseUrl: env.databaseUrl, runLabel: run },
    join(runDir, "accounts.json"),
  );
  log(`accounts ready: ${accounts.map((a) => a.handle).join(", ")}`);
  if (cmd === "bootstrap") return;

  const wanted = personas.map((p) => wantsBrain(p, cfg));
  if (wanted.includes("llm") && !env.anthropicKey) throw new Error("ANTHROPIC_API_KEY is not set and at least one player has an LLM brain");
  if (wanted.includes("jev") && !env.openrouterKey) throw new Error("OPENROUTER_API_KEY is not set and at least one player has a Jev brain");

  const budget = new Budget(cfg.budget.max_tokens, cfg.budget.max_usd);
  // What kind of society this is, read once through an unrecorded client so a recording holds only the player's own exchanges.
  const facts = await readFacts(makeClient({ baseUrl: env.ismsUrl, auth: { key: accounts[0]!.key } }), env.society);
  if (facts.preset !== values.preset) log(`note: the society was seeded from ${facts.preset}, not ${values.preset}; the persona list is ${values.preset}'s`);
  // SJ.1 lays out Freeport's choices only; the Commune's slots (ballots, offices, the Plan) are a follow-up.
  if (wanted.includes("jev") && hasAssembly(facts)) throw new Error(`the Jev brain plays Freeport only for now; ${facts.display} has an assembly`);
  log(`society: ${facts.display} (${facts.preset}); money ${facts.money}, labor ${facts.labor}, governance ${facts.governance}, store ${facts.common_store}, offices ${facts.offices.join(", ") || "none"}`);
  const players = accounts.map((a, i) => {
    const persona = personas[i]!;
    const name = a.handle;
    let transport: Transport = liveTransport;
    if (recording === name) {
      const file = join(runDir, `${name}.recording.jsonl`);
      writeFileSync(file, "");
      transport = recordingTransport(liveTransport, file);
      log(`recording ${name} to ${file}`);
    }
    const client = makeClient({ baseUrl: env.ismsUrl, auth: { key: a.key }, transport });
    const mk = () => new Anthropic({ apiKey: env.anthropicKey, fetch: transport, maxRetries: 2 });
    return new Player({
      run,
      name,
      persona,
      brain: brainFor(persona, cfg, mk, budget, name, a.handle, facts, { key: env.openrouterKey, transport }),
      client,
      sid: env.society,
      maxActions: cfg.turn.max_actions_per_turn,
      journal: new Journal(join(runDir, `${name}.journal.jsonl`)),
      notes: new Notes(join(runDir, `${name}.notes.md`)),
    });
  });

  // The tick watcher's own clock reads go through an unrecorded client: a recording holds only the player's turns (SJ.1 found the startup read shifting a replay by one).
  const clockClient = makeClient({ baseUrl: env.ismsUrl, auth: { key: accounts[0]!.key } });
  const readClock = async () => unwrap(await clockClient.GET("/s/{id}/home", { params: { path: { id: env.society } } })).clock;
  const aborter = new AbortController();
  process.on("SIGINT", () => {
    log("stopping after the turns in flight");
    aborter.abort();
  });
  let ticks: AsyncIterable<TickSignal> = watchTicks({
    baseUrl: env.ismsUrl,
    society: env.society,
    key: accounts[0]!.key,
    readClock,
    pollMs: cfg.run.poll_ms,
    signal: aborter.signal,
  });
  if (recording) {
    const signalsFile = join(runDir, "signals.jsonl");
    writeFileSync(signalsFile, "");
    const inner = ticks;
    ticks = (async function* () {
      for await (const s of inner) {
        appendFileSync(signalsFile, JSON.stringify(s) + "\n");
        yield s;
      }
    })();
  }

  const client = makeClient({ baseUrl: env.ismsUrl, auth: { key: accounts[0]!.key } });
  const summary = await runCohort(
    { run, runDir, players, budget, concurrency: cfg.turn.concurrency, overrun: cfg.turn.overrun, client, sid: env.society, log },
    ticks,
  );
  log(`done: ${summary.turns} turns, ${summary.skipped} skipped, $${summary.budget.total.usd.toFixed(2)}, stopped by ${summary.stopped_by}`);
  const md = buildReport(fold(runDir, run));
  writeFileSync(join(runDir, "report.md"), md);
  mkdirSync(REPORTS_DIR, { recursive: true });
  const file = join(REPORTS_DIR, `${run}.md`);
  writeFileSync(file, md);
  log(`report: ${file}`);
  // A last read so the society's own log shows the run ended with the players present.
  await unwrap(await client.GET("/s/{id}/home", { params: { path: { id: env.society } } }));
}

main(process.argv.slice(2)).catch((e) => {
  console.error(e instanceof Error ? (e.stack ?? e.message) : String(e));
  process.exit(1);
});
