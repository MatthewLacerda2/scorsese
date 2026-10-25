// A prompt written signed out, on the landing page, through the sign-in and
// into a new project already running it (#1535) — with its file, once, and
// with the prompt left in the chat box when there is no credit.

import { fileURLToPath } from "node:url";
import { expect, type Page, test } from "@playwright/test";
import { fakeAssistant } from "./fakeAssistant";
import { fakeGoogle } from "./session";

const STILL = fileURLToPath(new URL("./still.png", import.meta.url));

/** Writes `prompt` in the landing's first box, with `files` attached, and sends it. */
async function promptSignedOut(page: Page, prompt: string, files: string[] = []) {
  await fakeGoogle(page);
  await page.goto("/");
  const box = page.getByRole("textbox", { name: "Describe your video" }).first();
  if (files.length > 0) {
    await page.locator('input[type="file"]').first().setInputFiles(files);
    await expect(page.getByTitle("still.png").first()).toBeVisible();
  }
  await box.fill(prompt);
  await box.press("Enter");
  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
}

test("a landing prompt goes through the sign-in into a project running it, with its file, once", async ({
  page,
}) => {
  await fakeAssistant(page);
  const messages: { prompt: string; attachments: number[] }[] = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && /\/api\/projects\/\d+\/chat$/.test(request.url()))
      messages.push(request.postDataJSON());
  });
  const prompt = "a windmill festival in springtime";
  await promptSignedOut(page, prompt, [STILL]);
  await expect(page.getByText(prompt)).toBeVisible();
  expect(messages).toHaveLength(1);
  expect(messages[0]?.attachments).toHaveLength(1);

  // The hand-off again, as a reload of it: nothing is made twice.
  await page.goto("/start");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("link", { name: "Windmill Festival Springtime" })).toHaveCount(1);
  expect(messages).toHaveLength(1);
});

test("with no credit, the project opens with the prompt waiting in its chat box", async ({
  page,
}) => {
  await page.route(/\/api\/projects\/\d+\/chat$/, (route, request) =>
    request.method() === "POST"
      ? route.fulfill({ status: 402, json: { error: "You have no credit left." } })
      : route.fallback(),
  );
  await promptSignedOut(page, "a harbour at night");
  await expect(page.getByText("You have no credit left for the assistant.")).toBeVisible();
  await expect(page.getByPlaceholder(/^Ask the assistant/)).toHaveValue("a harbour at night");
});
