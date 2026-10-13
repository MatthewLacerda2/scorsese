// What the site tells crawlers, held to the app it describes: robots.txt keeps
// every signed-in page out of search results, and security.txt stays in date.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

describe("robots.txt", () => {
  const disallowed = read("../../public/robots.txt")
    .split("\n")
    .filter((line) => line.startsWith("Disallow:"))
    .map((line) => line.slice("Disallow:".length).trim());

  test("disallows every page but the landing page", () => {
    // Each `path="/…"` in routes.tsx is a page; its first segment is what a
    // Disallow line has to cover. `/` is the landing page, the one to crawl.
    const pages = [...read("./routes.tsx").matchAll(/path="(\/[^/"]+)/g)].map(
      (m) => m[1] as string,
    );
    expect(pages.length).toBeGreaterThan(0);
    for (const page of pages) expect(disallowed).toContain(page);
  });

  test("disallows the API", () => {
    expect(disallowed).toContain("/api/");
  });

  test("allows the landing page", () => {
    expect(disallowed).not.toContain("/");
  });
});

describe("security.txt", () => {
  test("expires no sooner than two months from now", () => {
    // RFC 9116 makes Expires required and a lapsed file one to ignore. This
    // fails while there is still time to renew it: push the date a year on.
    const expires = read("../../public/.well-known/security.txt").match(/^Expires: (.+)$/m)?.[1];
    expect(expires).toBeDefined();
    const twoMonths = 60 * 24 * 60 * 60 * 1000;
    expect(Date.parse(expires as string) - Date.now()).toBeGreaterThan(twoMonths);
  });
});
