// The assistant's Markdown renders as elements, and its untrusted text never
// as HTML (#768).

import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Reply } from "./Reply";

const shown = (text: string) => renderToStaticMarkup(<Reply text={text} />);

test("emphasis, bold, a list and a code block render as elements", () => {
  const html = shown("_like this_ and **THIS WAY**\n\n- one\n- two\n\n```\ncut 0:04\n```");
  expect(html).toContain("<em>like this</em>");
  expect(html).toContain("<strong>THIS WAY</strong>");
  expect(html).toMatch(/<ul[^>]*>\s*<li>one<\/li>/);
  expect(html).toMatch(/<pre[^>]*><code[^>]*>cut 0:04/);
});

test("raw HTML in the text renders as text", () => {
  const html = shown("hi <script>alert(1)</script>\n\n<img src=x onerror=alert(1)>");
  expect(html).not.toContain("<script");
  expect(html).not.toContain("<img");
  expect(html).toContain("&lt;script&gt;");
  expect(html).toContain("&lt;img src=x onerror=alert(1)&gt;");
});

test("a Markdown image is never fetched: its alt text shows instead", () => {
  const html = shown("look: ![a cat](https://example.com/cat.png)");
  expect(html).not.toContain("<img");
  expect(html).not.toContain("cat.png");
  expect(html).toContain("a cat");
});

test("a link opens in a new tab without a way back, and a script link goes nowhere", () => {
  const html = shown("[docs](https://example.com) [x](javascript:alert(1))");
  expect(html).toContain('href="https://example.com" target="_blank" rel="noopener noreferrer"');
  expect(html).not.toContain("javascript:");
});

test("half a reply mid-stream renders without throwing", () => {
  expect(shown("Making it **bigg")).toContain("**bigg");
  expect(shown("Here:\n\n```\nunfinished")).toContain("unfinished");
});

test("a heading stays the chat's size", () => {
  expect(shown("# Plan")).not.toContain("<h1");
});
