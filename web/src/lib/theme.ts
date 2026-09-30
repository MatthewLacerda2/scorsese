// Light or dark, and which one the user chose.
//
// The choice is a `dark` class on `<html>` (shadcn's variant, index.css) and
// is remembered in `localStorage`. Until the user has chosen, the page follows
// the system's `prefers-color-scheme`. The class is first set by the inline
// script in `index.html`, before React loads, so a dark page never flashes
// white; that script cannot import this module, so it repeats `resolveTheme`
// in a few lines and `theme.test.ts` runs it to hold the two together.
//
// `localStorage` can throw — a private window, storage disabled — so every
// access is caught, and a failure reads as "not chosen".

import { useCallback, useEffect, useState } from "react";

export type Theme = "light" | "dark";

/** The `localStorage` key; `index.html`'s script reads the same one. */
export const THEME_KEY = "scorsese-theme";

const SYSTEM_DARK = "(prefers-color-scheme: dark)";

/** What the user chose, or `null` if nothing (or storage cannot be read). */
export function storedTheme(storage: Pick<Storage, "getItem"> | undefined): Theme | null {
  try {
    const value = storage?.getItem(THEME_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null;
  }
}

/** Remember a choice; a storage that refuses only means it is not remembered. */
export function saveTheme(storage: Pick<Storage, "setItem"> | undefined, theme: Theme): void {
  try {
    storage?.setItem(THEME_KEY, theme);
  } catch {
    // Private window or storage disabled: the choice lasts this page only.
  }
}

/** The stored choice wins; otherwise the system's preference. */
export function resolveTheme(stored: Theme | null, systemDark: boolean): Theme {
  return stored ?? (systemDark ? "dark" : "light");
}

/** Put a theme on the page: the class Tailwind reads, and native controls' scheme. */
export function applyTheme(root: HTMLElement, theme: Theme): void {
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
}

/** `localStorage`, or `undefined` where even reading the property throws. */
function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

/** The theme the page shows, and a toggle that remembers the new one. */
export function useTheme(): { theme: Theme; toggle: () => void } {
  // `index.html` has already put the class on; read it back rather than
  // resolving again. Without a document (a test's server render) it is light.
  const [theme, setTheme] = useState<Theme>(() =>
    typeof document !== "undefined" && document.documentElement.classList.contains("dark")
      ? "dark"
      : "light",
  );

  // Until the user chooses, follow the system as it changes too.
  useEffect(() => {
    if (storedTheme(browserStorage()) !== null) return;
    const query = window.matchMedia(SYSTEM_DARK);
    const follow = () => {
      if (storedTheme(browserStorage()) !== null) return;
      const next = resolveTheme(null, query.matches);
      applyTheme(document.documentElement, next);
      setTheme(next);
    };
    query.addEventListener("change", follow);
    return () => query.removeEventListener("change", follow);
  }, []);

  const toggle = useCallback(() => {
    const next: Theme = theme === "dark" ? "light" : "dark";
    saveTheme(browserStorage(), next);
    applyTheme(document.documentElement, next);
    setTheme(next);
  }, [theme]);

  return { theme, toggle };
}
