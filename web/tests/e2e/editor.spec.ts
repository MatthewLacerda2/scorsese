// A file from upload to picture: into the library, into the project, onto the
// timeline, and drawn by the server into the preview.

import { expect, test } from "@playwright/test";
import { newProject, placeStill } from "./session";

test("an uploaded file reaches the library, the assets, the timeline and the preview", async ({
  page,
}) => {
  await newProject(page, "Upload");

  await placeStill(page);

  // The server draws the frame under the playhead as a still — a picture with
  // pixels in it, not a broken image the right size. Nothing is rendered
  // until play is pressed (#1448); then the preview video is the picture.
  const still = page.getByAltText("The frame under the playhead");
  const width = () => still.evaluate((shown) => (shown as HTMLImageElement).naturalWidth);
  await expect.poll(width, { timeout: 30_000 }).toBeGreaterThan(0);
  await page.getByRole("button", { name: "Play" }).click();
  const video = page.locator("video");
  const playing = () => video.evaluate((shown) => (shown as HTMLVideoElement).videoWidth);
  await expect.poll(playing, { timeout: 60_000 }).toBeGreaterThan(0);
  await expect(video).toBeVisible();

  // And the library page has the file too.
  await page.goto("/library");
  await expect(page.getByTitle("still.png", { exact: true })).toBeVisible();
});
