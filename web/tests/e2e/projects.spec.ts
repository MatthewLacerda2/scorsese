// Logging in, and making a project: the first two things anybody does.

import { expect, test } from "@playwright/test";
import { logIn, newProject } from "./session";

test("logging in lands on the projects list", async ({ page }) => {
  await logIn(page);
  await expect(page).toHaveURL(/\/projects$/);
});

test("a wrong password is refused, and the page says so", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("nobody@example.com");
  await page.getByLabel("Password").fill("not-the-password");
  await page.getByRole("button", { name: "Log in" }).click();
  await expect(page).toHaveURL(/\/login/);
  await expect(page.getByRole("alert")).toBeVisible();
});

test("creating a project opens its editor, and the list then has it", async ({ page }) => {
  await newProject(page, "Lighthouse");
  await page.goto("/projects");
  await expect(page.getByText("Lighthouse")).toBeVisible();
});
