// Before any flow: an account and a session to sign in with, made the way the
// operator makes them (docs/web.md, *Accounts*) — `scorsese-server user
// create`, then `user session` for its cookie — never a row inserted by hand,
// and never through Google, which no test reaches (#952). Kept where the
// flows read it.

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const accountFile = () => join(process.env.SCORSESE_E2E_RUN ?? "", "account.json");

export default function setup() {
  const run = process.env.SCORSESE_E2E_RUN ?? "";
  const server = resolve(process.env.SCORSESE_SERVER ?? "../target/debug/scorsese-server");
  // As the server is started (playwright.config.ts): away from any `.env`.
  const operator = (...args: string[]) =>
    execFileSync(server, ["user", ...args], {
      encoding: "utf8",
      cwd: run,
      env: {
        ...process.env,
        SCORSESE_STORAGE: join(run, "storage"),
        SCORSESE_CACHE: join(run, "cache"),
      },
    });
  // A fresh address each run, so a database a run has used before still takes it.
  const email = `e2e-${Date.now()}@example.com`;
  operator("create", email);
  const said = operator("session", email);
  const session = /scorsese_session=(\S+)/.exec(said)?.[1];
  if (!session) throw new Error(`user session printed no cookie:\n${said}`);
  writeFileSync(accountFile(), JSON.stringify({ email, session }));
}
