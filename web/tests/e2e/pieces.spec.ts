// A cut over thirty seconds plays in pieces from the playhead (#1448): nothing
// is made until play is pressed, then a few seconds at a time arrive, each
// with its own sound, and the video plays straight across the joins.

import { expect, type Page, test } from "@playwright/test";
import { newProject, placeStill } from "./session";

/** Calls an editor tool on project `id` at `revision` as the page's own session. */
async function tool(
  page: Page,
  id: string,
  revision: number,
  name: string,
  args: Record<string, unknown>,
) {
  const answer = await page.evaluate(
    async (call) => {
      const response = await fetch(`/api/projects/${call.id}/tools/${call.name}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ arguments: call.args, revision: call.revision }),
      });
      return `${response.status} ${await response.text()}`;
    },
    { id, revision, name, args },
  );
  expect(answer).toMatch(/^200 /);
}

test("a cut over thirty seconds plays as soon as its first piece lands", async ({ page }) => {
  await newProject(page, "Pieces");
  await placeStill(page);
  const id = /\/projects\/(\d+)\//.exec(page.url())?.[1] ?? "";
  const project = await page.evaluate(
    async (which) => (await fetch(`/api/projects/${which}`)).json(),
    id,
  );
  const [clip] = project.document.tracks[0].clips;
  const fps = project.document.timeline_fps;
  const end = ((clip.start + clip.duration) * fps.den) / fps.num;
  await tool(page, id, project.revision, "place_clip", {
    asset: clip.asset,
    track: project.document.tracks[0].id,
    start_seconds: end,
    duration_seconds: 35,
  });
  // The edit reaches the editor: the clock's total is past thirty seconds.
  await expect(page.getByText(/ \/ 0:(3[5-9]|[4-5]\d)$/)).toBeVisible();

  const asked: { range?: string }[] = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && request.url().endsWith("/previews"))
      asked.push(request.postDataJSON());
  });
  // The edit alone makes no preview.
  await page.waitForTimeout(1500);
  expect(asked).toEqual([]);

  await page.getByRole("button", { name: "Play", exact: true }).click();
  const video = page.locator("video");
  const at = () => video.evaluate((shown) => (shown as HTMLVideoElement).currentTime);
  // Across the first join, five seconds in, without stopping.
  await expect.poll(at, { timeout: 60_000 }).toBeGreaterThan(6);
  await expect(video).toBeVisible();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  const ranges = asked.map((body) => body.range);
  expect(ranges).toContain("0:150");
  expect(ranges).toContain("150:300");
  // Never the whole cut at once.
  expect(ranges.every(Boolean)).toBe(true);
});
