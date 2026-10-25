// The first test (#1554): the landing's heading in each version and language.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { draw } from "@/ab/draw";
import { TESTS } from "@/ab/tests";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import type { Language } from "@/i18n/language";
import { Hero } from "@/landing/Hero";

const HEADINGS: Record<Language, { a: string; b: string }> = {
  en: { a: "The video is one conversation away", b: "Describe the video. We'll make it." },
  "pt-BR": {
    a: "O vídeo está a uma conversa de distância",
    b: "Descreva o vídeo. A gente monta.",
  },
  es: {
    a: "El video está a una conversación de distancia",
    b: "Describe el video. Nosotros lo armamos.",
  },
};

function heading(language: Language, version?: string) {
  const html = renderToString(
    <QueryClientProvider client={createQueryClient()}>
      <I18nProvider initial={language}>
        <Hero onPrompt={async () => {}} version={version} />
      </I18nProvider>
    </QueryClientProvider>,
  );
  return /<h1[^>]*>(.*?)<\/h1>/.exec(html)?.[1] ?? "";
}

test("the heading test is defined, running, and draws both versions", () => {
  const hero = TESTS.find((t) => t.id === "hero-title");
  expect(hero).toEqual({ id: "hero-title", versions: ["a", "b"], running: true });
  const seen = new Set(
    Array.from({ length: 40 }, (_, i) => (hero ? draw(`visitor-${i}`, hero) : "")),
  );
  expect([...seen].sort()).toEqual(["a", "b"]);
});

for (const language of Object.keys(HEADINGS) as Language[]) {
  test(`version a is today's heading in ${language}`, () => {
    expect(heading(language, "a")).toBe(HEADINGS[language].a);
    expect(heading(language)).toBe(HEADINGS[language].a);
  });
  test(`version b is the promise in ${language}`, () => {
    expect(heading(language, "b").replace("&#x27;", "'")).toBe(HEADINGS[language].b);
  });
}
