// What every flow starts from: the run's account (setup.ts), logged in.

import { readFileSync } from "node:fs";
import { expect, type Page } from "@playwright/test";
import { accountFile } from "./setup";

export function account(): { email: string; password: string } {
  return JSON.parse(readFileSync(accountFile(), "utf8"));
}

/** Logs in through the login page, and waits for the projects list. */
export async function logIn(page: Page) {
  const { email, password } = account();
  await page.goto("/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
}

/** Logs in and creates a project named `name`, and waits for its editor. */
export async function newProject(page: Page, name: string) {
  await logIn(page);
  await page.getByPlaceholder("Project's name").fill(name);
  await page.getByRole("button", { name: "Create" }).click();
  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
  await expect(page.getByRole("heading", { name: "Assets" })).toBeVisible();
}
