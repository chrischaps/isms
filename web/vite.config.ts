/// <reference types="vitest/config" />
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The API the dev server proxies to (`make dev` serves it on 8080).
const api = process.env.ISMS_API ?? "http://127.0.0.1:8080";
// Anchored regexes: a bare "/s" prefix would also catch "/src/main.tsx".
const proxy = Object.fromEntries(
  ["^/auth/", "^/me$", "^/me/", "^/societies", "^/s/[0-9]+", "^/healthz", "^/openapi.json", "^/docs"].map(
    (p) => [p, { target: api, changeOrigin: false, ws: true }],
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
