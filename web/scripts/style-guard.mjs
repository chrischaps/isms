// The style bible's one hard rule (docs/style.md §4.5, §11): components read
// tokens only. This fails `pnpm check` on a colour literal, a `dark:` variant,
// or a pre-Companion class name outside the LEGACY allowlist below. The
// allowlist shrinks as screens are restyled and is emptied by the sweep (SB.5).

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("../src/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const SKIP = [/[\\/]styles[\\/]tokens\.css$/, /[\\/]styles[\\/]components\.css$/, /\.test\.tsx?$/, /[\\/]api[\\/]schema\.d\.ts$/];

// Files still carrying the old names, until their card lands. Remove a path
// here when you restyle it; the sweep removes the whole list.
const LEGACY_FILES = new Set([
  "screens/Admin.tsx",
  "screens/Archives.tsx",
  "screens/Assembly.tsx",
  "screens/Contracts.tsx",
  "screens/Event.tsx",
  "screens/Ledger.tsx",
  "screens/Login.tsx",
  "screens/Onboarding.tsx",
  "screens/Org.tsx",
  "screens/Orgs.tsx",
  "screens/Profile.tsx",
  "screens/Public.tsx",
  "screens/Societies.tsx",
  "screens/SocietyScreen.tsx",
  "screens/Store.tsx",
  "screens/Talk.tsx",
  "screens/roles/BallotBuilder.tsx",
  "screens/roles/Coordinator.tsx",
  "components/DiffSinceLastSeen.tsx",
  "lib/markdown.tsx",
]);

const RULES = [
  { name: "hex colour", re: /(?<![\w&])#[0-9a-f]{3,8}\b/gi, legacyOk: false },
  { name: "rgb()/hsl()", re: /\b(?:rgb|hsl)a?\(/g, legacyOk: false },
  { name: "dark: variant", re: /(?<![\w-])dark:/g, legacyOk: false },
  {
    name: "legacy class",
    re: /(?<![\w-])(?:text|bg|border|fill|stroke|decoration)-(?:bad|warn|paper|paper-2|ink-2|accent-2)(?![\w-])/g,
    legacyOk: true,
  },
  { name: "legacy helper class", re: /(?<![\w-])(?:num|rule|explain)(?=["'\s`])/g, legacyOk: true, classOnly: true },
];

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else if (/\.(tsx?|css)$/.test(name)) out.push(p);
  }
  return out;
}

let failures = 0;
for (const file of walk(ROOT)) {
  if (SKIP.some((s) => s.test(file))) continue;
  const rel = relative(ROOT, file).replaceAll("\\", "/");
  const legacy = LEGACY_FILES.has(rel);
  const text = readFileSync(file, "utf8");
  const lines = text.split("\n");
  lines.forEach((line, i) => {
    // Comments may quote a hex or a class name when explaining a rule.
    const code = line.replace(/\/\/.*$/, "").replace(/\/\*.*?\*\//g, "");
    for (const r of RULES) {
      if (r.classOnly && !/className|class=/.test(code)) continue;
      r.re.lastIndex = 0;
      const m = r.re.exec(code);
      if (!m) continue;
      if (r.legacyOk && legacy) continue;
      failures++;
      console.error(`${rel}:${i + 1}: ${r.name} "${m[0]}" — use a token (docs/style.md §4)`);
    }
  });
}

if (failures) {
  console.error(`\nstyle-guard: ${failures} problem${failures === 1 ? "" : "s"}.`);
  process.exit(1);
}
console.log("style-guard: clean.");
