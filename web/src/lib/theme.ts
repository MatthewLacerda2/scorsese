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
// Stored through `lib/storage.ts`, which catches a storage that throws — a
// private window, storage disabled — so a failure reads as "not chosen".

import { useCallback, useEffect, useState } from "react";
import { browserStorage, type Codec, KEYS, read, write } from "./storage";

export type Theme = "light" | "dark";

/** What the user picks: a theme, or "follow the system" (the default). */
export type ThemeChoice = Theme | "system";

const SYSTEM_DARK = "(prefers-color-scheme: dark)";

/** A theme is stored; System is no stored value at all. */
const CODEC: Codec<ThemeChoice> = {
  fallback: "system",
  parse: (raw) => (raw === "light" || raw === "dark" ? raw : undefined),
  format: (choice) => (choice === "system" ? null : choice),
};

/** The stored choice, read as one of the three the control shows. */
export function storedChoice(storage: Pick<Storage, "getItem"> | undefined): ThemeChoice {
  return read(storage, KEYS.theme, CODEC);
}

/** What the user chose, or `null` if nothing (or storage cannot be read). */
export function storedTheme(storage: Pick<Storage, "getItem"> | undefined): Theme | null {
  const choice = storedChoice(storage);
  return choice === "system" ? null : choice;
}

/**
 * Remember a choice: a theme is stored, System removes the stored one. A
 * storage that refuses only means it is not remembered.
 */
export function saveChoice(
  storage: Pick<Storage, "setItem" | "removeItem"> | undefined,
  choice: ThemeChoice,
): void {
  write(storage, KEYS.theme, CODEC, choice);
}

/** The stored choice wins; otherwise the system's preference. */
export function resolveTheme(stored: Theme | null, systemDark: boolean): Theme {
  return stored ?? (systemDark ? "dark" : "light");
}

/** Put a theme on the page: the class Tailwind reads, and native controls' scheme. */
function applyTheme(root: HTMLElement, theme: Theme): void {
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
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
