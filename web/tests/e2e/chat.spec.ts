// "Full" (#1027): the chat takes the editor's page and gives it back, a draft
// and the panels' place survive the switch, and a reload opens the project
// in the mode it was left in.

import { expect, test } from "@playwright/test";
import { newProject } from "./session";

test("Full gives the chat the page and back, keeping the draft and the mode", async ({ page }) => {
  await newProject(page, "Full chat");
  const draft = page.getByPlaceholder(/Ask the assistant/);
  const handles = page.getByRole("separator");
  await draft.fill("cut the intro");
  await expect(handles).toHaveCount(3);

  await page.getByRole("button", { name: "Full", exact: true }).click();
  await expect(handles).toHaveCount(0);
  await expect(draft).toHaveValue("cut the intro");

  await page.reload();
  await expect(page.getByRole("button", { name: "Back to the editor" })).toBeVisible();
  await expect(handles).toHaveCount(0);

  await page.getByRole("button", { name: "Back to the editor" }).click();
  await expect(handles).toHaveCount(3);
});
