// The landing's A/B tests (#1553), on the build's stand-in test: visitors are
// drawn into both versions, a reload keeps one's version, and a visit's view,
// sections, visible time and prompt reach the operator's report.

import { execFileSync } from "node:child_process";
import { join, resolve } from "node:path";
import { type BrowserContext, expect, type Page, test } from "@playwright/test";
import { fakeGoogle } from "./session";

const TEST = "e2e-fixture";

/** A browser that is not driven, as far as the page can tell, carrying `code`. */
async function visitor(context: BrowserContext, code: string) {
  await context.addInitScript(() =>
    Object.defineProperty(navigator, "webdriver", { get: () => false }),
  );
  await context.addCookies([
    { name: "scorsese_visitor", value: code, domain: "127.0.0.1", path: "/", sameSite: "Lax" },
  ]);
}

/** The version the landing drew, from its `data-ab`. */
async function drawn(page: Page): Promise<string> {
  await page.goto("/");
  const ab = await page.locator(".landing").getAttribute("data-ab");
  return /e2e-fixture:(\S+)/.exec(ab ?? "")?.[1] ?? "";
}

/** `scorsese-server ab report` for the stand-in test, as the operator runs it. */
function report(): string {
  const run = process.env.SCORSESE_E2E_RUN ?? "";
  const server = resolve(process.env.SCORSESE_SERVER ?? "../target/debug/scorsese-server");
  return execFileSync(server, ["ab", "report", "--test", TEST], {
    encoding: "utf8",
    cwd: run,
    env: {
      ...process.env,
      SCORSESE_STORAGE: join(run, "storage"),
      SCORSESE_CACHE: join(run, "cache"),
    },
  });
}

test("visitors are drawn into both versions, and a reload keeps each one's", async ({
  browser,
}) => {
  const seen = new Map<string, number>();
  for (let at = 0; at < 10; at += 1) {
    const context = await browser.newContext();
    await visitor(context, `5c1e0000-0000-4000-8000-${String(at).padStart(12, "0")}`);
    const page = await context.newPage();
    const version = await drawn(page);
    expect(await drawn(page)).toBe(version);
    seen.set(version, (seen.get(version) ?? 0) + 1);
    await context.close();
  }
  expect([...seen.keys()].sort()).toEqual(["a", "b"]);
  for (const count of seen.values()) expect(count).toBeGreaterThanOrEqual(3);
});

test("a visit's view, depth, time and prompt reach the report", async ({ browser }) => {
  const context = await browser.newContext();
  await visitor(context, "5c1e0000-0000-4000-8000-0000000000ff");
  const page = await context.newPage();
  const kinds: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith("/api/ab/events"))
      for (const event of request.postDataJSON().events) kinds.push(event.kind);
  });
  await fakeGoogle(page);
  const version = await drawn(page);
  await page.locator("footer").scrollIntoViewIfNeeded();
  await expect.poll(() => kinds).toContain("end");
  await page.waitForTimeout(1200);
  // The page hidden, as a switched tab: its visible seconds go out.
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  const box = page.getByRole("textbox", { name: "Describe your video" }).first();
  await box.fill("a paper boat on a pond");
  await box.press("Enter");
  await expect(page).toHaveURL(/\/projects\/\d+\/edit$/);
  expect(new Set(kinds)).toEqual(new Set(["view", "section", "end", "time", "prompt"]));
  await context.close();

  // What reached the server: this version's prompt and last-section shares are no longer nil.
  const row = () =>
    report()
      .split("\n")
      .find((line) => line.trim().startsWith(`${version} `))
      ?.trim()
      .split(/\s+/) ?? [];
  await expect.poll(() => row()[4]).not.toBe("0%");
  expect(row()[2]).not.toBe("0%");
});
