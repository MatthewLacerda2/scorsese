// Before any flow: an account to log in with, made the way the operator makes
// one (docs/web.md, *No public sign-up*) — `scorsese-server user create`,
// never a row inserted by hand — and kept where the flows read it.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const accountFile = () => join(process.env.SCORSESE_E2E_RUN ?? "", "account.json");

export default function setup() {
  const run = process.env.SCORSESE_E2E_RUN ?? "";
  const server = resolve(process.env.SCORSESE_SERVER ?? "../target/debug/scorsese-server");
  // A fresh address each run, so a database a run has used before still takes it.
  const email = `e2e-${Date.now()}@example.com`;
  const said = execFileSync(server, ["user", "create", email], {
    encoding: "utf8",
    // As the server is started (playwright.config.ts): away from any `.env`.
    cwd: run,
    env: {
      ...process.env,
      SCORSESE_STORAGE: join(run, "storage"),
      SCORSESE_CACHE: join(run, "cache"),
    },
  });
  const password = /password: (\S+)/.exec(said)?.[1];
  if (!password) throw new Error(`user create printed no password:\n${said}`);
  writeFileSync(accountFile(), JSON.stringify({ email, password }));
}
