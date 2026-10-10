// "Save as template" with nothing selected (#1005): unavailable, and saying
// why where a person looks — on hover, and on a tap where there is no hover.
// A `disabled` button takes no pointer events, so its tooltip never showed.

import { expect, test } from "@playwright/test";
import { newProject } from "./session";

const WHY = "Templates are selected clips, kept to reuse.";

test("an unavailable Save as template explains itself on hover and on a tap", async ({ page }) => {
  await newProject(page, "Templates");
  const button = page.getByRole("button", { name: "Save as template" });
  const why = page.getByRole("tooltip");
  await expect(button).toHaveAttribute("aria-disabled", "true");
  await expect(why).toBeHidden();

  await button.hover();
  await expect(why).toContainText(WHY);
  await page.mouse.move(0, 0);
  await expect(why).toBeHidden();

  // A tap: what a touch screen has instead of a hover. No dialog opens.
  // `force`, because Playwright waits for an `aria-disabled` button to be
  // enabled; a browser still delivers the click.
  await button.click({ force: true });
  await page.mouse.move(0, 0);
  await expect(why).toContainText(WHY);
  await expect(page.getByRole("dialog")).toBeHidden();
});
