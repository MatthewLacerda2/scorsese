// A cut longer than thirty seconds plays in pieces (#1448): nothing is made
// until play is pressed, then the picture arrives a few seconds at a time
// from the playhead and plays straight across the joins.

import { fileURLToPath } from "node:url";
import { expect, type Page, test } from "@playwright/test";
import { newProject } from "./session";

const STILL = fileURLToPath(new URL("./still.png", import.meta.url));

/** A project holding one still, stretched to a minute on the timeline. */
async function aMinuteOfStill(page: Page) {
  await newProject(page, "A minute");
  await page.getByRole("button", { name: "Library" }).click();
  const modal = page.getByRole("dialog");
  await modal.locator('input[type="file"]').setInputFiles(STILL);
  await modal.getByTitle("still.png", { exact: true }).click();
  await page.keyboard.press("Escape");
  const tile = page.getByTitle("Drag onto a track to place it");
  await tile.dragTo(page.getByText("Drag something here from the right."));
  const clip = page.getByRole("button", { name: /still/ }).and(page.locator("[data-made]"));
  await clip.click();
  const duration = page.getByText("Duration", { exact: true }).locator("+ label input");
  await duration.fill("60");
  await duration.press("Enter");
  await expect(page.getByText("0:00 / 1:00")).toBeVisible();
}

test("a long cut plays in pieces from the playhead, made only on play", async ({ page }) => {
  const asked: string[] = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && request.url().endsWith("/previews"))
      asked.push(request.postData() ?? "");
  });
  await aMinuteOfStill(page);
  // Edits alone make no preview.
  await page.waitForTimeout(1500);
  expect(asked).toEqual([]);

  await page.getByRole("button", { name: "Play", exact: true }).click();
  const video = page.locator("video:visible");
  await expect(video).toBeVisible({ timeout: 30_000 });
  // Across the first join, five seconds in, without stopping.
  const at = () => video.evaluate((element) => (element as HTMLVideoElement).currentTime);
  await expect.poll(at, { timeout: 30_000 }).toBeGreaterThan(6);
  await page.getByRole("button", { name: "Pause", exact: true }).click();

  const ranges = asked.map((body) => JSON.parse(body).range);
  expect(ranges).toContain("0:150");
  expect(ranges).toContain("150:300");
  // Never the whole minute at once.
  expect(ranges.every(Boolean)).toBe(true);
});
