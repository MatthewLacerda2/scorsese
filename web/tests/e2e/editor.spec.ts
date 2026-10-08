// A file from upload to picture: into the library, into the project, onto the
// timeline, and drawn by the server into the preview.

import { join } from "node:path";
import { expect, test } from "@playwright/test";
import { newProject } from "./session";

const STILL = join(import.meta.dirname, "still.png");

test("an uploaded file reaches the library, the assets, the timeline and the preview", async ({
  page,
}) => {
  await newProject(page, "Upload");

  // Uploaded through the editor's library modal, as a person would mid-edit.
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

  // Dragged onto the empty timeline, it is a clip on a new lane.
  await tile.dragTo(page.getByText("Drag something here from the left."));
  const clip = page.getByRole("button", { name: /still/ }).and(page.locator("[data-made]"));
  await expect(clip).toBeVisible();

  // The server draws the frame under the playhead: a still, or the preview
  // video once it is ready — either way a picture with pixels in it, not a
  // broken image the right size.
  const frame = page.getByAltText("The frame under the playhead").or(page.locator("video"));
  const pixels = () =>
    frame.evaluate((shown) =>
      shown instanceof HTMLImageElement
        ? shown.naturalWidth
        : (shown as HTMLVideoElement).videoWidth,
    );
  await expect.poll(pixels, { timeout: 30_000 }).toBeGreaterThan(0);

  // And the library page has the file too.
  await page.goto("/library");
  await expect(page.getByTitle("still.png", { exact: true })).toBeVisible();
});
