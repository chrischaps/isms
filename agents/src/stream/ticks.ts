// When to take a turn. The server's `/s/{id}/stream` WebSocket sends a frame per
// committed batch with the clock; a `TickResolved` in it means the tick moved,
// a `CycleClosed` means a cycle ended. When the socket cannot be had, the
// clock on `/home` is polled instead. Either way the consumer sees one
// `TickSignal` per edge, never the raw frames.

import WebSocket from "ws";
import type { Clock, StreamFrame } from "../api/client.ts";

export type TickSignal = {
  kind: "tick" | "cycle" | "epoch_end";
  clock: Clock;
  /** How the edge was seen. */
  via: "ws" | "poll";
};

export type ClockSource = () => Promise<Clock>;

export type WatchOpts = {
  baseUrl: string;
  society: number;
  key: string;
  /** `/home`'s clock, for the poll fallback and the first reading. */
  readClock: ClockSource;
  pollMs: number;
  /** Injected in tests. */
  makeSocket?: (url: string, headers: Record<string, string>) => WebSocket;
  signal?: AbortSignal;
};

/** Which edges lie between two clock readings. */
export function edgesBetween(prev: Clock | null, next: Clock): TickSignal["kind"][] {
  const out: TickSignal["kind"][] = [];
  if (prev === null) return out;
  // Once the epoch has ended there is no hour to play: only that edge is reported.
  if (next.epoch_ended) {
    if (!prev.epoch_ended) out.push("epoch_end");
    return out;
  }
  const moved = next.epoch !== prev.epoch || next.engine_tick !== prev.engine_tick;
  if (!moved) return out;
  if (next.cycle !== prev.cycle || next.epoch !== prev.epoch) out.push("cycle");
  out.push("tick");
  return out;
}

/** The queue between the source of frames and the consumer of signals. */
class Signals {
  private queue: TickSignal[] = [];
  private waiters: ((s: TickSignal | null) => void)[] = [];
  private closed = false;
  push(s: TickSignal) {
    const w = this.waiters.shift();
    if (w) w(s);
    else this.queue.push(s);
  }
  close() {
    this.closed = true;
    for (const w of this.waiters.splice(0)) w(null);
  }
  next(): Promise<TickSignal | null> {
    const s = this.queue.shift();
    if (s) return Promise.resolve(s);
    if (this.closed) return Promise.resolve(null);
    return new Promise((resolve) => this.waiters.push(resolve));
  }
}

/** Frames and polls both come here; it emits each edge once. */
export class EdgeDetector {
  private last: Clock | null = null;
  private readonly out: (s: TickSignal) => void;
  constructor(out: (s: TickSignal) => void) {
    this.out = out;
  }
  seen(clock: Clock, via: TickSignal["via"]) {
    for (const kind of edgesBetween(this.last, clock)) this.out({ kind, clock, via });
    this.last = clock;
  }
  get clock(): Clock | null {
    return this.last;
  }
}

/** Frames carry the clock; the kinds inside are only a cross-check for the log. */
export function frameClock(frame: StreamFrame): Clock {
  return frame.clock;
}

export async function* watchTicks(opts: WatchOpts): AsyncGenerator<TickSignal> {
  const signals = new Signals();
  const edges = new EdgeDetector((s) => signals.push(s));
  edges.seen(await opts.readClock(), "poll");

  let socket: WebSocket | null = null;
  let pollTimer: NodeJS.Timeout | null = null;
  let stopped = false;

  const stop = () => {
    stopped = true;
    if (pollTimer) clearInterval(pollTimer);
    socket?.close();
    signals.close();
  };
  opts.signal?.addEventListener("abort", stop, { once: true });

  const startPolling = () => {
    if (pollTimer || stopped) return;
    pollTimer = setInterval(() => {
      opts.readClock().then(
        (c) => edges.seen(c, "poll"),
        () => undefined,
      );
    }, opts.pollMs);
  };

  const connect = () => {
    if (stopped) return;
    const url = `${opts.baseUrl.replace(/^http/, "ws")}/s/${opts.society}/stream`;
    const headers = { authorization: `Bearer ${opts.key}` };
    socket = opts.makeSocket ? opts.makeSocket(url, headers) : new WebSocket(url, { headers });
    socket.on("open", () => {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    });
    socket.on("message", (data) => {
      try {
        const frame = JSON.parse(data.toString()) as StreamFrame;
        edges.seen(frameClock(frame), "ws");
      } catch {
        // A malformed frame is the server's bug, not a reason to stop; the poll will catch the edge.
      }
    });
    socket.on("error", () => undefined);
    socket.on("close", () => {
      socket = null;
      if (stopped) return;
      startPolling();
      setTimeout(connect, opts.pollMs);
    });
  };
  connect();

  try {
    for (;;) {
      const s = await signals.next();
      if (s === null) return;
      yield s;
    }
  } finally {
    stop();
  }
}
