// Arriving, signing in, and making a project: the first things anybody does.

import { expect, test } from "@playwright/test";
import { logIn, newProject } from "./session";

test("signed in, /login and / both land on the projects list", async ({ page }) => {
  await logIn(page);
  await expect(page).toHaveURL(/\/projects$/);
  await page.goto("/");
  await expect(page).toHaveURL(/\/projects$/);
});

test("a first visit is the landing page, and its Sign in goes to the server's Google sign-in", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByText("It gets made.")).toBeVisible();
  await page.getByRole("button", { name: "Sign in" }).first().click();
  await page.getByRole("dialog").getByRole("link", { name: "Continue with Google" }).click();
  // This run's server has no Google client (playwright.config.ts), so the
  // real route answers as an unconfigured one does, and the popup says so.
  await expect(page).toHaveURL(/\/login\?error=unconfigured$/);
  await expect(page.getByRole("alert")).toHaveText("Sign-in isn't set up on this server yet.");
});

test("creating a project opens its editor, and the list then has it", async ({ page }) => {
  await newProject(page, "Lighthouse");
  await page.goto("/projects");
  await expect(page.getByText("Lighthouse")).toBeVisible();
});

test("a project started for a platform and a style opens upright, and says so", async ({
  page,
}) => {
  await logIn(page);
  await page.getByRole("button", { name: "Create" }).click();
  const modal = page.getByRole("dialog");
  await modal.getByLabel("Project's name").fill("Flash sale");
  await modal.getByRole("button", { name: "3. Platform" }).click();
  await expect(modal.getByText("You can choose or change the platform later.")).toBeVisible();
  await modal.getByRole("radio", { name: /TikTok ad/ }).click();
  await modal.getByRole("button", { name: "Next" }).click();
  await expect(modal.getByText("You can choose or change the style later.")).toBeVisible();
  // Only the styles made for a TikTok ad, each with a placeholder until #1017.
  await expect(modal.getByRole("radio", { name: /Board that draws itself/ })).toHaveCount(0);
  await expect(modal.getByTestId("style-placeholder").first()).toBeVisible();
  await modal.getByRole("radio", { name: /Flash offer/ }).click();
  await modal.getByRole("button", { name: "Create" }).click();

  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
  await expect(page.getByRole("combobox", { name: "Frame shape" })).toHaveValue("portrait");
  await expect(page.getByRole("button", { name: "Platform and style" })).toContainText(
    "Flash offer",
  );
});
