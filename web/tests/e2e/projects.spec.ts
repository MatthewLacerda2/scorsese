// Arriving, signing in, and making a project: the first things anybody does.

import { expect, test } from "@playwright/test";
import { fakeAssistant } from "./fakeAssistant";
import { logIn, newProject } from "./session";

test("signed in, /login and / both land on the home, and /projects too", async ({ page }) => {
  await logIn(page);
  await expect(page).toHaveURL(/\/$/);
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("textbox", { name: "Describe your video" })).toBeVisible();
});

test("a first visit is the landing page, and its Sign in goes to the server's Google sign-in", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Create your videos right now" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Sign in" }).first().click();
  await page.getByRole("dialog").getByRole("link", { name: "Continue with Google" }).click();
  // This run's server has no Google client (playwright.config.ts), so the
  // real route answers as an unconfigured one does, and the popup says so.
  await expect(page).toHaveURL(/\/login\?error=unconfigured$/);
  await expect(page.getByRole("alert")).toHaveText("Sign-in isn't set up on this server yet.");
});

test("a prompt on the home makes a named project, opens its editor and says the prompt there", async ({
  page,
}) => {
  await fakeAssistant(page);
  await logIn(page);
  await page.getByRole("textbox", { name: "Describe your video" }).fill("a lighthouse at dawn");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
  await expect(page.getByText("a lighthouse at dawn")).toBeVisible();
  // The project is in the sidebar of the home, named from the prompt.
  await page.getByRole("link", { name: "Home" }).click();
  await expect(page.getByRole("link", { name: "Lighthouse Dawn" })).toBeVisible();
});

test("a new project's editor asks nothing about what it is for", async ({ page }) => {
  await newProject(page, "Lighthouse");
  // The assistant reads the platform, shape and kind from the prompt (#1531).
  await expect(page.getByRole("combobox", { name: "Frame shape" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Platform and style" })).toHaveCount(0);
});
