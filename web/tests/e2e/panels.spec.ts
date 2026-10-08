// A panel dragged to a new size keeps it through a reload (#863): the size is
// the browser's, in `localStorage`, not the project's.

import { expect, test } from "@playwright/test";
import { newProject } from "./session";

test("a resized panel keeps its size through a reload", async ({ page }) => {
  await newProject(page, "Panels");
  const handle = page.getByRole("separator", { name: /assets/i });
  const before = Number(await handle.getAttribute("aria-valuenow"));

  const box = await handle.boundingBox();
  if (!box) throw new Error("the assets handle has no box");
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  // The assets are on the right (#943): their edge grows dragged leftward.
  await page.mouse.move(x - 48, y, { steps: 4 });
  await page.mouse.up();
  const after = Number(await handle.getAttribute("aria-valuenow"));
  expect(after).toBeCloseTo(before + 3, 0);

  await page.reload();
  await expect(page.getByRole("separator", { name: /assets/i })).toHaveAttribute(
    "aria-valuenow",
    String(after),
  );
});
