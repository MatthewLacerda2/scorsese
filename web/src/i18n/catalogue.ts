// The message catalogues, and the type that holds them together.
//
// English is the source: `Messages` is its shape, so every other catalogue is
// typed against it and a missing or misspelt key is a type error, not a
// silent fallback. A message is a string, or a function when it carries a
// value (`(name: string) => …`) — the arguments are type-checked too, and a
// plural is just a function of a number. `catalogue.test.ts` checks the same
// at run time, for the day a cast slips past the compiler.

import { en } from "@/i18n/en";
import { es } from "@/i18n/es";
import type { Language } from "@/i18n/language";
import { ptBR } from "@/i18n/pt-BR";

/** Every user-visible string, in one language. */
export type Messages = typeof en;

/** One catalogue per language. */
export const CATALOGUES: Record<Language, Messages> = { en, "pt-BR": ptBR, es };

/**
 * A catalogue with English under it: any message it lacks reads in English.
 * The types make that impossible today; this keeps a gap from rendering as
 * `undefined` if one ever gets past them.
 */
export function withFallback<T>(chosen: T, fallback: T): T {
  if (typeof fallback !== "object" || fallback === null) return chosen ?? fallback;
  const merged: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(fallback)) {
    const own = (chosen as Record<string, unknown> | undefined)?.[key];
    merged[key] = withFallback(own, value);
  }
  return merged as T;
}

/** The messages a language reads in, English filling any gap. */
export function messagesFor(language: Language): Messages {
  return language === "en" ? en : withFallback(CATALOGUES[language], en);
}
