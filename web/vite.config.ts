/// <reference types="vitest/config" />
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The API the dev server proxies to (`make dev` serves it on 8080).
const api = process.env.ISMS_API ?? "http://127.0.0.1:8080";
// Anchored regexes: a bare "/s" prefix would also catch "/src/main.tsx".
// Client routes share the `/s/{id}` prefix with the API (S1.9 adds
// `/s/{id}/work` and `/s/{id}/plan`), so a page load or reload there (a
// navigation, which asks for HTML) gets the app; fetches get the API.
// The deploy's reverse proxy (S1.14) needs the same rule.
const bypass = (req: { headers: { accept?: string; upgrade?: string } }) =>
  req.headers.upgrade === undefined && req.headers.accept?.includes("text/html") ? "/index.html" : undefined;
const proxy = Object.fromEntries(
  ["^/auth/", "^/me$", "^/me/", "^/societies", "^/s/[0-9]+", "^/public/", "^/admin/", "^/healthz", "^/openapi.json", "^/docs"].map(
    (p) => [p, { target: api, changeOrigin: false, ws: true, ...(p.startsWith("^/s") || p.startsWith("^/public") ? { bypass } : {}) }],
  ),
);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: { proxy },
  preview: { proxy },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
