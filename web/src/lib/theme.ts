// Light or dark, and which one the user chose.
//
// The theme is a `dark` class on `<html>` (shadcn's variant, index.css). The
// user's choice is Light, Dark or System; Light and Dark are remembered in
// `localStorage`, and System is the absence of a stored value — so choosing it
// clears the key, and the page follows the system's `prefers-color-scheme`. The class is first set by the inline
// script in `index.html`, before React loads, so a dark page never flashes
// white; that script cannot import this module, so it repeats `resolveTheme`
// in a few lines and `theme.test.ts` runs it to hold the two together.
//
// `localStorage` can throw — a private window, storage disabled — so every
// access is caught, and a failure reads as "not chosen".

import { useCallback, useEffect, useState } from "react";

export type Theme = "light" | "dark";

/** What the user picks: a theme, or "follow the system" (the default). */
export type ThemeChoice = Theme | "system";

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

/** The stored choice, read as one of the three the control shows. */
export function storedChoice(storage: Pick<Storage, "getItem"> | undefined): ThemeChoice {
  return storedTheme(storage) ?? "system";
}

/**
 * Remember a choice: a theme is stored, System removes the stored one. A
 * storage that refuses only means it is not remembered.
 */
export function saveChoice(
  storage: Pick<Storage, "setItem" | "removeItem"> | undefined,
  choice: ThemeChoice,
): void {
  try {
    if (choice === "system") storage?.removeItem(THEME_KEY);
    else storage?.setItem(THEME_KEY, choice);
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

/** The theme the page shows, the user's choice, and a way to change it. */
export function useTheme(): {
  theme: Theme;
  choice: ThemeChoice;
  choose: (choice: ThemeChoice) => void;
} {
  // `index.html` has already put the class on; read it back rather than
  // resolving again. Without a document (a test's server render) it is light.
  const [theme, setTheme] = useState<Theme>(() =>
    typeof document !== "undefined" && document.documentElement.classList.contains("dark")
      ? "dark"
      : "light",
  );
  const [choice, setChoice] = useState<ThemeChoice>(() => storedChoice(browserStorage()));

  // While the choice is System, follow the system as it changes too.
  useEffect(() => {
    if (choice !== "system") return;
    const query = window.matchMedia(SYSTEM_DARK);
    const follow = () => {
      const next = resolveTheme(null, query.matches);
      applyTheme(document.documentElement, next);
      setTheme(next);
    };
    follow();
    query.addEventListener("change", follow);
    return () => query.removeEventListener("change", follow);
  }, [choice]);

  const choose = useCallback((next: ThemeChoice) => {
    saveChoice(browserStorage(), next);
    setChoice(next);
    if (next !== "system") {
      applyTheme(document.documentElement, next);
      setTheme(next);
    }
  }, []);

  return { theme, choice, choose };
}
