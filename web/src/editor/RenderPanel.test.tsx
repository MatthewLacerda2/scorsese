// The Render dialog's choices (#1070), drawn: the kind of file, size, rate and
// loudness, each on the default the server would pick, in every language.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { createQueryClient } from "@/app/queryClient";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { RenderPanel } from "./RenderPanel";

const draw = (language: (typeof LANGUAGES)[number]["language"]) =>
  renderToString(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial={language}>
        <RenderPanel projectId={4} name="teaser" aspect="9:16" fps={{ num: 30000, den: 1001 }} />
      </I18nProvider>
    </QueryClientProvider>,
  );

test("every choice is offered on its default, in the reader's language", () => {
  for (const { language } of LANGUAGES) {
    const words = CATALOGUES[language].editor.render;
    const html = draw(language);
    for (const label of [words.file, words.resolution, words.rate, words.loudness])
      expect(html).toContain(`aria-label="${label}"`);
    expect(html).toMatch(/<option [^>]*value="mp4" selected="">/);
    expect(html).toContain(words.sound("mp3"));
    expect(html).toMatch(/<option [^>]*value="1080x1920" selected="">/);
    expect(html).toContain(words.loudnesses.streaming);
  }
  // The edit's own rate is named, as a person reads it.
  expect(draw("en")).toContain("Same as the edit (29.97 fps)");
  expect(draw("pt-BR")).toContain("29,97 fps");
});
