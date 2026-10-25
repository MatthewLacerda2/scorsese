// The preview's transport on a phone (#1578): at a phone's width every
// control of it is on the screen, none clipped past its right edge.

import { expect, test } from "@playwright/test";
import { newProject } from "./session";

test("a phone's video view shows the whole transport", async ({ page }) => {
  // Made at the desktop width the helpers expect, then turned into a phone.
  await newProject(page, "Phone");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Show the video" }).click();

  const controls = [
    page.getByRole("button", { name: "Play" }),
    page.getByRole("slider", { name: "Scrub" }),
    page.getByText(/^\d+:\d\d \/ \d+:\d\d$/),
    page.getByRole("combobox", { name: "Preview quality" }),
  ];
  for (const control of controls) {
    await expect(control).toBeVisible();
    const box = await control.boundingBox();
    if (!box) throw new Error("a transport control has no box");
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
});
