// Light or dark, and which one the user chose.
//
// The theme is a `dark` class on `<html>` (shadcn's variant, index.css). Light
// is the default: the user's choice, Light or Dark, is remembered in
// `localStorage`, and with nothing stored the page is light, whatever the
// system prefers. The class is first set by the inline script in `index.html`,
// before React loads, so a dark page never flashes white; that script cannot
// import this module, so it repeats the rule in a few lines and `theme.test.ts`
// runs it to hold the two together.
//
// Stored through `lib/storage.ts`, which catches a storage that throws — a
// private window, storage disabled — so a failure reads as "not chosen".

import { useCallback, useState } from "react";
import { browserStorage, type Codec, KEYS, oneOf, read, write } from "./storage";

export type Theme = "light" | "dark";

/** Light unless the user chose dark. */
const CODEC: Codec<Theme> = oneOf(["light", "dark"], "light");

/** The stored theme, or light when nothing is stored (or storage cannot be read). */
export function storedTheme(storage: Pick<Storage, "getItem"> | undefined): Theme {
  return read(storage, KEYS.theme, CODEC);
}

/** Remember a choice. A storage that refuses only means it is not remembered. */
export function saveTheme(
  storage: Pick<Storage, "setItem" | "removeItem"> | undefined,
  theme: Theme,
): void {
  write(storage, KEYS.theme, CODEC, theme);
}

/** Put a theme on the page: the class Tailwind reads, and native controls' scheme. */
function applyTheme(root: HTMLElement, theme: Theme): void {
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
}

/** The theme the page shows, and a way to change it. */
export function useTheme(): { theme: Theme; choose: (theme: Theme) => void } {
  // `index.html` has already put the class on; read it back rather than
  // resolving again. Without a document (a test's server render) it is light.
  const [theme, setTheme] = useState<Theme>(() =>
    typeof document !== "undefined" && document.documentElement.classList.contains("dark")
      ? "dark"
      : "light",
  );
  const choose = useCallback((next: Theme) => {
    saveTheme(browserStorage(), next);
    applyTheme(document.documentElement, next);
    setTheme(next);
  }, []);
  return { theme, choose };
}
