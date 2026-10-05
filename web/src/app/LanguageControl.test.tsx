// The picker shows the chosen language's flag beside its name (#770), drawn
// inline — nothing fetched, no emoji Windows would spell out as letters.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import { LanguageControl } from "./LanguageControl";

function picker(language: Language) {
  return renderToString(
    <I18nProvider initial={language}>
      <LanguageControl />
    </I18nProvider>,
  );
}

test("the closed picker shows the chosen language's flag and name", () => {
  const pt = picker("pt-BR");
  expect(pt).toContain('data-flag="pt-BR"');
  expect(pt).toContain("Português (Brasil)");
  expect(pt).not.toContain('data-flag="es"');

  expect(picker("es")).toContain('data-flag="es"');
  expect(picker("en")).toContain('data-flag="en"');
});

test("a flag is decoration, hidden from screen readers, and fetches nothing", () => {
  const html = picker("en");
  expect(html).toMatch(/<svg[^>]*aria-hidden="true"[^>]*data-flag="en"/);
  expect(html).not.toMatch(/<img|href=|url\(/);
});
