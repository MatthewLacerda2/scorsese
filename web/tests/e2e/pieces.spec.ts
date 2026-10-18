// A cut over thirty seconds plays in pieces from the playhead (#1448): play
// asks for the sound and a few seconds of picture, and the video plays as
// soon as the first piece lands, without waiting for the whole cut.

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

  const asked: unknown[] = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && request.url().endsWith("/previews"))
      asked.push(request.postDataJSON());
  });
  await page.getByRole("button", { name: "Play" }).click();
  const video = page.locator("video");
  const at = () => video.evaluate((shown) => (shown as HTMLVideoElement).currentTime);
  await expect.poll(at, { timeout: 60_000 }).toBeGreaterThan(0.5);
  await expect(video).toBeVisible();
  expect(asked).toContainEqual(expect.objectContaining({ stream: "sound" }));
  expect(asked).toContainEqual(expect.objectContaining({ stream: "picture", range: "0:150" }));
  expect(asked).not.toContainEqual(expect.not.objectContaining({ stream: expect.anything() }));
});
