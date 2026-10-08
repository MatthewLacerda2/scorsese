// The end-to-end flows (#898): a few things a person does in the web app,
// driven in headless Chromium against the real `scorsese-server` and a real
// Postgres, so a front end and a server that disagree are caught. The
// assistant is the one thing faked, in the browser (`tests/e2e/assistant.spec.ts`):
// nothing here can spend money, and CI has no provider keys anyway.
//
// Run through `make web-e2e`, which builds the server and brings a Postgres;
// by hand it needs, in the environment:
//
//   DATABASE_URL       a Postgres the run may create tables in
//   SCORSESE_SERVER    the server binary (default: the workspace's debug build)
//   E2E_CHROMIUM       optional: a Chromium to use instead of Playwright's own
//
// The page is the production build (`vite build`, then `vite preview`), whose
// `/api` proxy (vite.config.ts) points at the server started here — the same
// one-origin shape the deploy's nginx gives.

import { mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { defineConfig, devices } from "@playwright/test";

const SERVER_PORT = 8091;
const PAGE_PORT = 4174;

// One directory for the run's files and account, made once by the main
// process: its workers inherit the variable rather than each making their own.
process.env.SCORSESE_E2E_RUN ??= mkdtempSync(join(tmpdir(), "scorsese-e2e-"));
const run = process.env.SCORSESE_E2E_RUN;
for (const dir of ["storage", "cache"]) mkdirSync(join(run, dir), { recursive: true });
const server = resolve(process.env.SCORSESE_SERVER ?? "../target/debug/scorsese-server");

export default defineConfig({
  testDir: "tests/e2e",
  globalSetup: "./tests/e2e/setup.ts",
  // A flow is a few seconds against a warm server; a minute is something stuck.
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? [["list"], ["github"]] : "list",
  use: {
    baseURL: `http://127.0.0.1:${PAGE_PORT}`,
    // Kept for a failure only, and uploaded by CI: the trace replays every
    // step, request and DOM; `npx playwright show-trace` opens it.
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    viewport: { width: 1440, height: 900 },
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
        launchOptions: { executablePath: process.env.E2E_CHROMIUM || undefined },
      },
    },
  ],
  webServer: [
    {
      command: server,
      // Started outside the checkout, so no `.env` there is read: a developer's
      // provider keys never reach a server driven by tests.
      cwd: run,
      url: `http://127.0.0.1:${SERVER_PORT}/api/health`,
      env: {
        SCORSESE_STORAGE: join(run, "storage"),
        SCORSESE_CACHE: join(run, "cache"),
        SCORSESE_BIND: `127.0.0.1:${SERVER_PORT}`,
      },
      reuseExistingServer: false,
      stdout: "pipe",
      stderr: "pipe",
      timeout: 60_000,
    },
    {
      command: `bun run build && bun run preview --host 127.0.0.1 --port ${PAGE_PORT} --strictPort`,
      url: `http://127.0.0.1:${PAGE_PORT}/`,
      env: { SCORSESE_API: `http://127.0.0.1:${SERVER_PORT}` },
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
