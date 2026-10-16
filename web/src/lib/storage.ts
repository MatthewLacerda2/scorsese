// Everything the web app keeps in the browser (#1212), and the one place it
// touches `localStorage`. The `noRawStorage` house rule refuses the global
// anywhere else (web/README.md, *House rules*).
//
// Every key is named in `KEYS`, so what a user's browser holds is one list and
// two choices cannot collide on a name. A caller pairs its key with a `Codec` —
// the value when nothing usable is stored, and how the stored text is read and
// written — and gets typed reads and writes back. `localStorage` can throw — a
// private window, site data blocked — so every access is caught here: a read
// that fails is the codec's fallback, and a write that fails only means the
// choice lasts this page.
//
// The key names are what browsers already hold: renaming one silently forgets
// every user's saved choice.

/** Every key the web app stores, by what it holds. */
export const KEYS = {
  /** The language the user picked (`i18n/language.ts`). */
  language: "scorsese-language",
  /** Light or dark; absent is System (`lib/theme.ts`). `index.html`'s script reads it too. */
  theme: "scorsese-theme",
  /** The editor's panel sizes, one for the browser (`editor/panels/sizes.ts`). */
  panels: "scorsese.panels",
  /** Preview quality, one for the browser (`editor/preview/quality.ts`). */
  previewQuality: "scorsese.preview-quality",
  /** A project's assistant effort (`editor/chat/effort.ts`). */
  effort: (project: number) => `scorsese-effort-${project}`,
  /** Whether a project's chat fills the page (`editor/chat/full.ts`). */
  chatFull: (project: number) => `scorsese-chat-full-${project}`,
  /** A project's frame shape (`editor/shape.ts`). */
  shape: (project: number) => `scorsese.shape.${project}`,
} as const;

/** The part of `Storage` a read needs; tests hand in a map or one that throws. */
export type ReadStorage = Pick<Storage, "getItem">;

/** The part of `Storage` a write needs; only a codec that removes its key needs `removeItem`. */
export type WriteStorage = Pick<Storage, "setItem"> & Partial<Pick<Storage, "removeItem">>;

/** `localStorage`, or `undefined` outside a browser or where even reading the property throws. */
export function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

/** How one stored value is read and written. */
export interface Codec<T> {
  /** The value when nothing is stored, the stored text is not one, or storage throws. */
  fallback: T;
  /** The stored text as a value, or `undefined` when it is not one. May throw. */
  parse: (raw: string) => T | undefined;
  /** The text to store, or `null` to remove the key. `String` when left out. */
  format?: (value: T) => string | null;
}

/** A codec for one of a fixed set of strings. */
export function oneOf<T extends string, F extends T | null = T>(
  values: readonly T[],
  fallback: F,
): Codec<T | F> {
  return { fallback, parse: (raw) => values.find((value) => value === raw) };
}

/** `key`'s value in `storage`, or the codec's fallback. Never throws. */
export function read<T>(storage: ReadStorage | undefined, key: string, codec: Codec<T>): T {
  try {
    const raw = storage?.getItem(key);
    return (raw == null ? undefined : codec.parse(raw)) ?? codec.fallback;
  } catch {
    return codec.fallback;
  }
}

/** Store `value` under `key`. A storage that refuses only means it is not remembered. */
export function write<T>(
  storage: WriteStorage | undefined,
  key: string,
  codec: Codec<T>,
  value: T,
): void {
  try {
    const raw = codec.format ? codec.format(value) : String(value);
    if (raw === null) storage?.removeItem?.(key);
    else storage?.setItem(key, raw);
  } catch {
    // Private window or storage disabled: the choice lasts this page only.
  }
}
