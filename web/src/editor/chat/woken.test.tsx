// A turn the server started once the work it ordered landed (#1441) has no
// message of the person's to show: one line in their language says why it
// started, and the server's own wording for the model never shows.

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { TurnView } from "@/api/chat";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import { Turn } from "./Turn";
import { empty, upsert } from "./transcript";

function page(language: Language, woken: boolean): string {
  const turn: TurnView = {
    ...({} as TurnView),
    id: 2,
    session: 1,
    project: 7,
    prompt: "Continued after the stills landed",
    woken,
    state: "answered",
    answer: "Both stills are in; the cut is done.",
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    quote_replies: [],
    questions: [],
  };
  const [shown] = upsert(empty(7), turn).entries;
  if (!shown) throw new Error("no turn");
  return renderToString(
    <I18nProvider initial={language}>
      <Turn entry={shown} />
    </I18nProvider>,
  );
}

test.each(["en", "pt-BR", "es"] as Language[])(
  "a woken turn in %s shows its line, not a prompt bubble",
  (language) => {
    const shown = page(language, true);
    expect(shown).toContain(CATALOGUES[language].chat.turn.woken);
    expect(shown).not.toContain("Continued after the stills landed");
    expect(shown).not.toContain("bg-muted");
  },
);

test("a turn the person started shows their words", () => {
  const shown = page("en", false);
  expect(shown).toContain("Continued after the stills landed");
  expect(shown).not.toContain(CATALOGUES.en.chat.turn.woken);
});
