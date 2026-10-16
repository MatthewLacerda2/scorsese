// The busy line mounted for real, effects and all, in every language: a
// static render never runs `useEffect`, which is how a list message reaching
// pt-BR as an object blanked the whole editor whenever a turn ran.

import { afterEach, describe, expect, test } from "bun:test";
import { act, cleanup, render } from "@testing-library/react";
import { messagesFor } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { Busy } from "./Busy";

afterEach(cleanup);

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  test("a running turn's status line mounts and shows one of its words", async () => {
    const { container } = render(
      <I18nProvider initial={language}>
        <Busy />
      </I18nProvider>,
    );
    await act(async () => {});
    const shown = container.textContent?.trim();
    expect(messagesFor(language).chat.turn.busy).toContain(shown ?? "");
  });
});
