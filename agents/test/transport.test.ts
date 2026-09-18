import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { canonical, fixtureName, fixtureTransport, jsonResponse, keyOf, readRecording, recordingTransport, replayTransport } from "../src/api/transport.ts";

describe("keys and names", () => {
  it("keys by method, path and query", () => {
    expect(keyOf("get", "http://x/s/1/home")).toBe("GET /s/1/home");
    expect(keyOf("GET", "http://x/s/1/prices?window=24")).toBe("GET /s/1/prices?window=24");
    expect(fixtureName("GET /s/1/books/share:3")).toBe("GET_s_1_books_share_3.json");
  });
  it("compares bodies canonically", () => {
    expect(canonical('{"b":1,"a":[{"y":2,"x":1}]}')).toBe(canonical('{ "a": [ {"x":1,"y":2} ], "b": 1 }'));
    expect(canonical("not json")).toBe("not json");
    expect(canonical(null)).toBeNull();
  });
});

describe("fixtureTransport", () => {
  it("serves a file by key and a problem when there is none", async () => {
    const dir = mkdtempSync(join(tmpdir(), "fx-"));
    writeFileSync(join(dir, "GET_s_1_home.json"), JSON.stringify({ status: 200, body: { hello: 1 } }));
    const t = fixtureTransport(dir, { "POST /s/1/orders": { status: 422, body: { title: "no" } } });
    const ok = await t("http://x/s/1/home");
    expect(ok.status).toBe(200);
    expect(await ok.json()).toEqual({ hello: 1 });
    const over = await t("http://x/s/1/orders", { method: "POST" });
    expect(over.status).toBe(422);
    const missing = await t("http://x/s/1/nothing");
    expect(missing.status).toBe(404);
    expect(((await missing.json()) as { detail: string }).detail).toContain("no fixture for GET /s/1/nothing");
  });
});

describe("recording and replay", () => {
  it("replays a recording in order and refuses a different request", async () => {
    const dir = mkdtempSync(join(tmpdir(), "rec-"));
    const file = join(dir, "r.jsonl");
    let n = 0;
    const inner = async () => jsonResponse(200, { n: n++ });
    const rec = recordingTransport(inner, file);
    await rec("http://x/s/1/home");
    await rec("http://x/s/1/orders", { method: "POST", body: JSON.stringify({ qty: 1, side: "bid" }), headers: { "content-type": "application/json" } });
    const lines = readRecording(file);
    expect(lines.map((l) => [l.method, l.target, l.reqBody])).toEqual([
      ["GET", "game", null],
      ["POST", "game", '{"qty":1,"side":"bid"}'],
    ]);
    expect(readFileSync(file, "utf8")).not.toContain("authorization");

    const rep = replayTransport(lines);
    expect(await (await rep("http://x/s/1/home")).json()).toEqual({ n: 0 });
    // Same body, different key order: still the same request.
    const r2 = await rep("http://x/s/1/orders", { method: "POST", body: JSON.stringify({ side: "bid", qty: 1 }) });
    expect(await r2.json()).toEqual({ n: 1 });
    expect(rep.remaining()).toBe(0);
    await expect(rep("http://x/s/1/home")).rejects.toThrow(/exhausted/);
    const rep2 = replayTransport(lines);
    await expect(rep2("http://x/s/1/books")).rejects.toThrow(/mismatch/);
  });
});
