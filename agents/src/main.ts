// The command line: bootstrap the accounts, run the cohort, fold a run into a
// report, or record one player for the replay test.
//
//   pnpm run -- --run <name> [--players N] [--brain scripted|llm|mixed] [--model id] [--cycle-model id] [--max-usd n] [--record <player>]
//   pnpm bootstrap -- --run <name>
//   pnpm report <run>
//
// Environment: ISMS_URL, ISMS_SOCIETY, ISMS_SERVER_BIN, DATABASE_URL, ANTHROPIC_API_KEY.

import Anthropic from "@anthropic-ai/sdk";
import { appendFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { makeClient, unwrap } from "./api/client.ts";
import { liveTransport, recordingTransport, type Transport } from "./api/transport.ts";
import { ensurePlayers } from "./bootstrap/accounts.ts";
import type { Brain, Persona } from "./brain/brain.ts";
import { AnthropicBrain } from "./brain/llm/anthropic.ts";
import { ScriptedBrain } from "./brain/scripted/index.ts";
import { Budget } from "./budget/budget.ts";
import { runCohort } from "./cohort/cohort.ts";
import { loadConfig, readEnv, type Config } from "./config.ts";
import { Journal } from "./journal/journal.ts";
import { Notes } from "./player/notes.ts";
import { loadPersona } from "./player/persona.ts";
import { Player } from "./player/player.ts";
import { buildReport, fold } from "./report/report.ts";
import { watchTicks, type TickSignal } from "./stream/ticks.ts";

const HERE = import.meta.dirname;
export const AGENTS_DIR = resolve(HERE, "..");
export const PERSONAS_DIR = join(AGENTS_DIR, "personas");
export const RUNS_DIR = join(AGENTS_DIR, "runs");
export const REPORTS_DIR = resolve(AGENTS_DIR, "..", "docs", "playtest", "runs");

export function personasFor(cfg: Config): Persona[] {
  const available = new Map(readdirSync(PERSONAS_DIR).filter((f) => f.endsWith(".md")).map((f) => [f.replace(/\.md$/, ""), join(PERSONAS_DIR, f)]));
  const out: Persona[] = [];
  for (let i = 0; i < cfg.run.players; i++) {
    const slug = cfg.run.personas[i % cfg.run.personas.length]!;
    const file = available.get(slug);
    if (!file) throw new Error(`no persona file for ${slug} in ${PERSONAS_DIR}`);
    out.push(loadPersona(file));
  }
  return out;
}

export function brainFor(persona: Persona, cfg: Config, mk: () => Anthropic, budget: Budget, player: string, handle: string): Brain {
  const wants = cfg.run.brain === "mixed" ? persona.brain : cfg.run.brain;
  if (wants === "scripted") return new ScriptedBrain(persona, handle);
  return new AnthropicBrain({ client: mk(), persona, player, cfg, budget });
}

function log(line: string) {
  const at = new Date().toISOString().slice(11, 19);
  console.log(`${at} ${line}`);
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
  if (cmd !== "run" && cmd !== "bootstrap" && cmd !== "record") throw new Error("usage: run | bootstrap | report | record");

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
      "max-usd": { type: "string" },
      record: { type: "string" },
    },
  });
  const run = values.run ?? new Date().toISOString().slice(0, 16).replace(/[-:T]/g, "");
  const brainArg = values.brain as Config["run"]["brain"] | undefined;
  const cfg = loadConfig(values.config!, {
    players: cmd === "record" ? 1 : values.players ? Number(values.players) : undefined,
    brain: brainArg,
    turnModel: values.model,
    cycleModel: values["cycle-model"],
    maxUsd: values["max-usd"] ? Number(values["max-usd"]) : undefined,
  });
  const env = readEnv();
  const runDir = join(RUNS_DIR, run);
  mkdirSync(runDir, { recursive: true });
  const personas = personasFor(cfg);
  const recording = cmd === "record" ? (values.record ?? `${personas[0]!.slug}-1`) : values.record;

  log(`run ${run}: ${cfg.run.players} players, brain ${cfg.run.brain}, turn model ${cfg.models.turn}, cycle model ${cfg.models.cycle}, society ${env.society} at ${env.ismsUrl}`);
  const accounts = await ensurePlayers(
    personas.map((p) => p.slug),
    { baseUrl: env.ismsUrl, society: env.society, serverBin: env.serverBin, databaseUrl: env.databaseUrl, runLabel: run },
    join(runDir, "accounts.json"),
  );
  log(`accounts ready: ${accounts.map((a) => a.handle).join(", ")}`);
  if (cmd === "bootstrap") return;

  const needsKey = personas.some((p) => (cfg.run.brain === "mixed" ? p.brain : cfg.run.brain) === "llm");
  if (needsKey && !env.anthropicKey) throw new Error("ANTHROPIC_API_KEY is not set and at least one player has an LLM brain");

  const budget = new Budget(cfg.budget.max_tokens, cfg.budget.max_usd);
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
      brain: brainFor(persona, cfg, mk, budget, name, a.handle),
      client,
      sid: env.society,
      maxActions: cfg.turn.max_actions_per_turn,
      journal: new Journal(join(runDir, `${name}.journal.jsonl`)),
      notes: new Notes(join(runDir, `${name}.notes.md`)),
    });
  });

  const first = players[0]!;
  const readClock = async () => (await first.home()).clock;
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
