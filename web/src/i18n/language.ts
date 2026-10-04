// Which language the page speaks, and which one the user chose.
//
// The choice lives in the browser (`localStorage`), never on the account: the
// server does not need to know (#704). Until the user picks one, the browser's
// own language decides — `pt*` is Brazilian Portuguese, `es*` Spanish, and
// anything else English, which is also the fallback for a missing message.
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

/** A browser language tag (`navigator.language`) read as one we speak. */
export function browserLanguage(tag: string | undefined): Language {
  const lower = tag?.toLowerCase() ?? "";
  if (lower.startsWith("pt")) return "pt-BR";
  if (lower.startsWith("es")) return "es";
  return "en";
}

/** The stored choice wins; otherwise the browser's language. */
export function resolveLanguage(stored: Language | null, browserTag: string | undefined): Language {
  return stored ?? browserLanguage(browserTag);
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
  const tag = typeof navigator === "undefined" ? undefined : navigator.language;
  return resolveLanguage(storedLanguage(browserStorage()), tag);
}
