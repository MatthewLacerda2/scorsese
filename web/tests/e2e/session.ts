// What every flow starts from: the run's account (setup.ts), signed in with
// the session the operator's command printed — the cookie a Google sign-in
// would have set (#952).

import { readFileSync } from "node:fs";
import { expect, type Page } from "@playwright/test";
import { accountFile } from "./setup";

function account(): { email: string; session: string } {
  return JSON.parse(readFileSync(accountFile(), "utf8"));
}

/** The session cookie and its hint, as the server sets them (docs/web.md, *Accounts*). */
export async function setSession(page: Page) {
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
    // The readable hint the server sets beside it (#1246), so `/` waits on the
    // session rather than drawing the landing page first.
    {
      name: "scorsese_signed_in",
      value: "1",
      domain: "127.0.0.1",
      path: "/",
      secure: true,
      sameSite: "Strict",
    },
  ]);
}

/** Signs in (`setSession`), then `/login`, which a signed-in visitor leaves for the home. */
export async function logIn(page: Page) {
  await setSession(page);
  await page.goto("/login");
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
}

/**
 * Logs in and makes a project named `name` — through the API from the page, as
 * the home's prompt box does, but without a prompt to send — and waits for its editor.
 */
export async function newProject(page: Page, name: string) {
  await logIn(page);
  const id = await page.evaluate(async (projectName) => {
    const response = await fetch("/api/projects", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ name: projectName }),
    });
    return (await response.json()).id as number;
  }, name);
  await page.goto(`/projects/${id}/edit`);
  // The assets sidebar's Library button: the panel lost its heading (#1446).
  await expect(page.getByRole("button", { name: "Library" })).toBeVisible();
}

/**
 * Stands in for Google, which no test reaches (#952): the server's sign-in
 * route sets the run's session and sends the browser to its `next`, as a
 * successful Google sign-in does.
 */
export async function fakeGoogle(page: Page) {
  await page.route(/\/api\/auth\/google(\?.*)?$/, async (route, request) => {
    await setSession(page);
    const next = new URL(request.url()).searchParams.get("next") ?? "/";
    return route.fulfill({ status: 302, headers: { location: next } });
  });
}
