// A player's running notes: a markdown file under the run, rewritten once a
// day by the cycle model (or left empty by a script). Capped so the prefix
// stays small.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export const NOTES_CAP = 1600;

export class Notes {
  private readonly file: string;
  private text: string;
  constructor(file: string) {
    this.file = file;
    this.text = existsSync(file) ? readFileSync(file, "utf8") : "";
  }
  get(): string {
    return this.text;
  }
  set(notes: string, plan: string) {
    const body = plan.trim() === "" ? notes.trim() : `${notes.trim()}\n\n## Plan for tomorrow\n${plan.trim()}`;
    this.text = body.slice(0, NOTES_CAP);
    mkdirSync(dirname(this.file), { recursive: true });
    writeFileSync(this.file, this.text + "\n");
  }
}
