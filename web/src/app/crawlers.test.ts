// What the site tells crawlers, held to the app it describes: robots.txt keeps
// every signed-in page out of search results, the sitemap lists every public
// one, and security.txt stays in date.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

/** The pages besides `/` that anyone may read, and search engines index: the pricing, the terms and the privacy policy (#547). */
const PUBLIC = ["/pricing", "/terms", "/privacy"];

describe("sitemap.xml", () => {
  test("lists the landing page and every public page", () => {
    const urls = [
      ...read("../../public/sitemap.xml").matchAll(/<loc>https:\/\/[^/]+(\/[^<]*)<\/loc>/g),
    ].map((m) => m[1] as string);
    expect(urls.sort()).toEqual(["/", ...PUBLIC].sort());
  });
});

describe("robots.txt", () => {
  const robots = read("../../public/robots.txt");
  const disallowed = robots
    .split("\n")
    .filter((line) => line.startsWith("Disallow:"))
    .map((line) => line.slice("Disallow:".length).trim());

  test("disallows every page but the public ones", () => {
    // Each `path="/…"` in routes.tsx is a page; its first segment is what a
    // Disallow line has to cover. `/` is the landing page, the one to crawl.
    const pages = [...read("./routes.tsx").matchAll(/path="(\/[^/"]+)/g)].map(
      (m) => m[1] as string,
    );
    expect(pages.length).toBeGreaterThan(0);
    for (const page of pages.filter((path) => !PUBLIC.includes(path))) {
      expect(disallowed).toContain(page);
    }
  });

  test("lets crawlers read the public pages", () => {
    for (const page of PUBLIC) expect(disallowed).not.toContain(page);
  });

  test("disallows the API, but the prices the pricing page renders from", () => {
    expect(disallowed).toContain("/api/");
    const allowed = robots
      .split("\n")
      .filter((line) => line.startsWith("Allow:"))
      .map((line) => line.slice("Allow:".length).trim());
    expect(allowed).toEqual(["/api/prices"]);
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
