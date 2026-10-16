// A page in the library (kind `html`) is named as a page and has no picture to wait for.

import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { FILE_KINDS } from "@/api";
import { en } from "@/i18n/en";
import { Thumbnail } from "./Thumbnail";

test("a page is a kind the library filters by, with a name in every catalogue", () => {
  expect(FILE_KINDS).toContain("html");
  expect(en.files.kinds.html).toBe("Page");
});

test("a page's tile shows the kind's icon and never asks for a thumbnail", () => {
  const html = renderToStaticMarkup(<Thumbnail src="/api/library/9/thumbnail" kind="html" />);
  expect(html).not.toContain("<img");
  expect(html).toContain("<svg");
});
