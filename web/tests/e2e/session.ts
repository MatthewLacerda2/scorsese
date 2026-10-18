// What every flow starts from: the run's account (setup.ts), signed in with
// the session the operator's command printed — the cookie a Google sign-in
// would have set (#952).

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, type Page } from "@playwright/test";
import { accountFile } from "./setup";

function account(): { email: string; session: string } {
  return JSON.parse(readFileSync(accountFile(), "utf8"));
}

/**
 * Signs in: the session cookie and its hint as the server sets them
 * (docs/web.md, *Accounts*), then `/login`, which a signed-in visitor leaves for the
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
  // The assets sidebar's Library button: the panel lost its heading (#1446).
  await expect(page.getByRole("button", { name: "Library" })).toBeVisible();
}

const STILL = fileURLToPath(new URL("./still.png", import.meta.url));

/**
 * Uploads `still.png` through the editor's library modal, as a person would
 * mid-edit, picks it into the project and drags it onto the empty timeline,
 * where it is a clip on a new lane.
 */
export async function placeStill(page: Page) {
  await page.getByRole("button", { name: "Library" }).click();
  const modal = page.getByRole("dialog");
  await modal.locator('input[type="file"]').setInputFiles(STILL);
  const file = modal.getByTitle("still.png", { exact: true });
  await expect(file).toBeVisible();

  // Picked, it is one of the project's assets; the modal stays for another
  // pick until it is closed.
  await file.click();
  await expect(modal.getByText("in this project")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(modal).toBeHidden();
  const tile = page.getByTitle("Drag onto a track to place it");
  await expect(tile).toBeVisible();

  await tile.dragTo(page.getByText("Drag something here from the right."));
  const clip = page.getByRole("button", { name: /still/ }).and(page.locator("[data-made]"));
  await expect(clip).toBeVisible();
}
