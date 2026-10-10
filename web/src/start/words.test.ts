// The platforms' and styles' words in every language (#1051), held to the
// server's list: `menu.json` is `GET /api/styles` as served, kept equal to the
// library by the server's own test (`make style-menu`). So a platform or style
// added to the library without its words here fails this gate, with no server
// running.

import { describe, expect, test } from "bun:test";
import type { StyleMenu } from "@/api";
import { CATALOGUES } from "@/i18n/catalogue";
import { LANGUAGES } from "@/i18n/language";
import { MENU } from "./fixture";
import { localized } from "./menu";
import served from "./menu.json";

const SERVED: StyleMenu = served;

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  const words = CATALOGUES[language].menu;

  test("names exactly the platforms the server lists", () => {
    const ids = SERVED.platforms.map((platform) => platform.id);
    expect(Object.keys(words.platforms).sort()).toEqual(ids.sort());
  });

  test("names and describes exactly the styles the server lists, each in ≤ 30 words", () => {
    const ids = SERVED.styles.map((style) => style.id);
    expect(Object.keys(words.styles).sort()).toEqual(ids.sort());
    for (const [id, { description }] of Object.entries(words.styles)) {
      const count = description.split(/\s+/).filter(Boolean).length;
      expect({ id, short: count <= 30 }).toEqual({ id, short: true });
    }
  });
});

test("pt-BR's words are the library's own", () => {
  const shown = localized(SERVED, CATALOGUES["pt-BR"].menu);
  expect(shown).toEqual(SERVED);
});

test("the menu reads in the catalogue's words, and an id it lacks keeps the server's", () => {
  const unknown = {
    id: "new_style",
    name: "Estilo novo",
    description: "Um estilo que o catálogo ainda não conhece.",
    platforms: ["youtube"],
    preview: null,
  };
  const shown = localized({ ...MENU, styles: [...MENU.styles, unknown] }, CATALOGUES.en.menu);
  expect(shown.platforms.map((platform) => platform.name)).toEqual(["YouTube", "TikTok ad"]);
  expect(shown.styles.map((style) => style.name)).toEqual([
    "Kinetic type",
    "Flash offer",
    "Top N list",
    "Estilo novo",
  ]);
  expect(shown.styles[1]?.description).toBe(CATALOGUES.en.menu.styles.flash_offer.description);
  expect(shown.styles[1]?.platforms).toEqual(["tiktok_ad"]);
});
