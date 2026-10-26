// The running test (#1593): the first screen's headline in every version, and
// the line under it in each version and language. The home shows version `a`.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { draw } from "@/ab/draw";
import { TESTS } from "@/ab/tests";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import { Hero } from "@/landing/Hero";

const SCREEN: Record<Language, { headline: string; a: string; b: string }> = {
  en: {
    headline: "Create your videos right now",
    a: "Describe the video. We'll make it.",
    b: "You don't need to know how to edit. We do it for you.",
  },
  "pt-BR": {
    headline: "Crie seus vídeos agora mesmo",
    a: "Descreva o vídeo. A gente monta.",
    b: "Você não precisa saber editar. A gente edita pra você.",
  },
  es: {
    headline: "Crea tus videos ahora mismo",
    a: "Describe el video. Nosotros lo armamos.",
    b: "No necesitas saber editar. Lo editamos por ti.",
  },
};

function hero(language: Language, version?: string) {
  const html = renderToString(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial={language}>
        <Hero onPrompt={async () => {}} version={version} />
      </I18nProvider>
    </QueryClientProvider>,
  ).replaceAll("&#x27;", "'");
  return {
    headline: /<h1[^>]*>(.*?)<\/h1>/.exec(html)?.[1] ?? "",
    line: /<p[^>]*data-subtitle[^>]*>(.*?)<\/p>/.exec(html)?.[1] ?? "",
    wordmark: html.includes(">scorsese<"),
  };
}

test("the subtitle test is the one running, and draws both versions", () => {
  expect(TESTS.find((t) => t.id === "hero-title")).toBeUndefined();
  const subtitle = TESTS.find((t) => t.id === "hero-subtitle");
  expect(subtitle).toEqual({ id: "hero-subtitle", versions: ["a", "b"], running: true });
  const seen = new Set(
    Array.from({ length: 40 }, (_, i) => (subtitle ? draw(`visitor-${i}`, subtitle) : "")),
  );
  expect([...seen].sort()).toEqual(["a", "b"]);
});

for (const language of Object.keys(SCREEN) as Language[]) {
  test(`version a keeps the old test's winner under the headline in ${language}`, () => {
    for (const shown of [hero(language, "a"), hero(language)]) {
      expect(shown.headline).toBe(SCREEN[language].headline);
      expect(shown.line).toBe(SCREEN[language].a);
    }
  });
  test(`version b says no editing is needed in ${language}`, () => {
    const shown = hero(language, "b");
    expect(shown.headline).toBe(SCREEN[language].headline);
    expect(shown.line).toBe(SCREEN[language].b);
  });
}

test("the first screen has the mark and no wordmark", () => {
  expect(hero("en").wordmark).toBe(false);
});
