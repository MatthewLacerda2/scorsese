// What every flow starts from: the run's account (setup.ts), signed in with
// the session the operator's command printed — the cookie a Google sign-in
// would have set (#952).

import { readFileSync } from "node:fs";
import { expect, type Page } from "@playwright/test";
import { accountFile } from "./setup";

export function account(): { email: string; session: string } {
  return JSON.parse(readFileSync(accountFile(), "utf8"));
}

/**
 * Signs in: the session cookie as the server sets it (docs/web.md,
 * *Accounts*), then `/login`, which a signed-in visitor leaves for the
 * projects list.
 */
export async function logIn(page: Page) {
  await page.context().addCookies([
    {
      name: "scorsese_session",
      value: account().session,
      // The page's host (playwright.config.ts), where the API answers too.
      domain: "127.0.0.1",
      path: "/api",
      httpOnly: true,
      secure: true,
      sameSite: "Strict",
    },
  ]);
  await page.goto("/login");
  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
}

/**
 * Logs in and creates a project named `name` through the new-project modal
 * (#1016) — the name alone, every other step skipped — and waits for its editor.
 */
export async function newProject(page: Page, name: string) {
  await logIn(page);
  await page.getByRole("button", { name: "Create" }).click();
  const modal = page.getByRole("dialog");
  await modal.getByLabel("Project's name").fill(name);
  await modal.getByRole("button", { name: "Create" }).click();
  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
  await expect(page.getByRole("heading", { name: "Assets" })).toBeVisible();
}
