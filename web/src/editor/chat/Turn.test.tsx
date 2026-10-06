// A turn shows the assistant's words and tools, never its thinking: a working
// turn shows one of its language's status words instead, and a finished one
// shows neither (#767).

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { TurnView } from "@/api/chat";
import type { ServerEvent } from "@/api/events";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import { Turn } from "./Turn";
import { apply, type Entry, empty, upsert } from "./transcript";

function turnView(state: TurnView["state"], answer: string | null): TurnView {
  return {
    ...({} as TurnView),
    id: 1,
    session: 1,
    project: 7,
    prompt: "cut the intro",
    state,
    answer,
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    questions: [],
  };
}

/** The turn, after a thinking note and a word from the model. */
function entry(state: TurnView["state"], answer: string | null): Entry {
  let shown = upsert(empty(7), turnView("running", null));
  const note = { type: "chat_progress", turn: 1, text: "Weighing the cut privately" };
  shown = apply(shown, note as unknown as ServerEvent);
  shown = apply(shown, { type: "chat_text", turn: 1, text: "Cutting **now**" });
  shown = upsert(shown, turnView(state, answer));
  const [first] = shown.entries;
  if (!first) throw new Error("no turn");
  return first;
}

function html(language: Language, shown: Entry): string {
  return renderToString(
    <I18nProvider initial={language}>
      <Turn entry={shown} />
    </I18nProvider>,
  );
}

test.each(["en", "pt-BR", "es"] as Language[])(
  "a working turn in %s shows a status word",
  (language) => {
    const page = html(language, entry("running", null));
    expect(page).not.toContain("Weighing the cut");
    expect(page).toContain("<strong>now</strong>");
    const words = CATALOGUES[language].chat.turn.busy;
    expect(words.some((word) => page.includes(word))).toBe(true);
  },
);

test("a finished turn shows neither its thinking nor a status word", () => {
  const page = html("en", entry("answered", "Cut the _intro_."));
  expect(page).not.toContain("Weighing the cut");
  expect(page).toContain("<em>intro</em>");
  expect(CATALOGUES.en.chat.turn.busy.some((word) => page.includes(word))).toBe(false);
});
