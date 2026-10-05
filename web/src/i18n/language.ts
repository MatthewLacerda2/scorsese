// Which language the page speaks, and which one the user chose.
//
// The choice lives in the browser (`localStorage`), never on the account: the
// server does not need to know (#704). Until the user picks one, the browser's
// preferred languages decide, in order (`navigator.languages`, which the
// browser takes from the system): the first that is `pt*`, `es*` or `en*` picks
// Brazilian Portuguese, Spanish or English. When none is, or the browser names
// none, the page opens in Brazilian Portuguese — the maintainer's call (#770).
// That is about which language a page opens in, and nothing else: a message
// missing from a catalogue still falls back to English's, the source catalogue
// the others are typed against.
//
// Wrapped as `lib/theme.ts` wraps the theme: `localStorage` can throw — a
// private window, storage disabled — so every access is caught, and a failure
// reads as "not chosen".

/** A language the web app speaks; also the locale `Intl` formats in. */
export type Language = "en" | "pt-BR" | "es";

/** Every language, each named in itself — a reader looks for their own word. */
export const LANGUAGES: readonly { language: Language; name: string }[] = [
  { language: "en", name: "English" },
  { language: "pt-BR", name: "Português (Brasil)" },
  { language: "es", name: "Español" },
];

/** The `localStorage` key. */
export const LANGUAGE_KEY = "scorsese-language";

function isLanguage(value: unknown): value is Language {
  return LANGUAGES.some(({ language }) => language === value);
}

/** What the user chose, or `null` if nothing (or storage cannot be read). */
export function storedLanguage(storage: Pick<Storage, "getItem"> | undefined): Language | null {
  try {
    const value = storage?.getItem(LANGUAGE_KEY);
    return isLanguage(value) ? value : null;
  } catch {
    return null;
  }
}

/** Remember a choice. A storage that refuses only means it is not remembered. */
export function saveLanguage(
  storage: Pick<Storage, "setItem"> | undefined,
  language: Language,
): void {
  try {
    storage?.setItem(LANGUAGE_KEY, language);
  } catch {
    // Private window or storage disabled: the choice lasts this page only.
  }
}

/** The language a page opens in when nothing else decides it (#770). */
export const FALLBACK_LANGUAGE: Language = "pt-BR";

/** One browser language tag read as a language we speak, or `null`. */
function spoken(tag: string): Language | null {
  const lower = tag.toLowerCase();
  if (lower.startsWith("pt")) return "pt-BR";
  if (lower.startsWith("es")) return "es";
  if (lower.startsWith("en")) return "en";
  return null;
}

/**
 * The browser's preferred language tags (`navigator.languages`, best first)
 * read as the first one we speak — Brazilian Portuguese when none is.
 */
export function browserLanguage(tags: readonly string[] | undefined): Language {
  for (const tag of tags ?? []) {
    const language = spoken(tag);
    if (language) return language;
  }
  return FALLBACK_LANGUAGE;
}

/** The stored choice wins; otherwise the browser's preferred languages. */
export function resolveLanguage(
  stored: Language | null,
  browserTags: readonly string[] | undefined,
): Language {
  return stored ?? browserLanguage(browserTags);
}

/** `localStorage`, or `undefined` where even reading the property throws. */
export function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

/** The language a page opens in, read from the browser it runs in. */
export function initialLanguage(): Language {
  return resolveLanguage(storedLanguage(browserStorage()), navigatorLanguages());
}

/**
 * `navigator.languages`, or the lone `navigator.language` where a browser
 * offers only that; `undefined` outside a browser.
 */
function navigatorLanguages(): readonly string[] | undefined {
  if (typeof navigator === "undefined") return undefined;
  if (navigator.languages?.length) return navigator.languages;
  return navigator.language ? [navigator.language] : undefined;
}
