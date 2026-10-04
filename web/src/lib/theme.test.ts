// Which theme a page opens in: the stored choice wins, the system decides
// otherwise, and a storage that throws reads as "not chosen". Both copies of
// that rule are held here — `theme.ts`, and the inline script in `index.html`
// that runs before the bundle — so they cannot drift apart.

import { describe, expect, test } from "bun:test";
import {
  resolveTheme,
  saveChoice,
  storedChoice,
  storedTheme,
  THEME_KEY,
  type Theme,
} from "@/lib/theme";

const storage = (value: string | null) => ({
  getItem: (key: string) => (key === THEME_KEY ? value : null),
});
const throwing = {
  getItem: (): string | null => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
  removeItem: () => {
    throw new Error("SecurityError");
  },
};

/** A storage that keeps what it is given, as `localStorage` does. */
function memory() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
    removeItem: (key: string) => void values.delete(key),
    values,
  };
}

describe("theme.ts", () => {
  test("a stored choice wins over the system", () => {
    expect(resolveTheme(storedTheme(storage("light")), true)).toBe("light");
    expect(resolveTheme(storedTheme(storage("dark")), false)).toBe("dark");
  });

  test("with nothing stored, the system decides", () => {
    expect(resolveTheme(storedTheme(storage(null)), true)).toBe("dark");
    expect(resolveTheme(storedTheme(storage(null)), false)).toBe("light");
  });

  test("a value that is not a theme is no choice", () => {
    expect(storedTheme(storage("purple"))).toBeNull();
  });

  test("storage that throws falls back to the system, and saving does not throw", () => {
    expect(storedTheme(throwing)).toBeNull();
    expect(storedTheme(undefined)).toBeNull();
    expect(resolveTheme(storedTheme(throwing), true)).toBe("dark");
    expect(() => saveChoice(throwing, "dark")).not.toThrow();
    expect(() => saveChoice(throwing, "system")).not.toThrow();
  });

  test("each of the three choices reads back as itself", () => {
    const store = memory();
    expect(storedChoice(store)).toBe("system");
    for (const choice of ["light", "dark", "system"] as const) {
      saveChoice(store, choice);
      expect(storedChoice(store)).toBe(choice);
    }
  });

  test("choosing System clears the stored choice, so the system decides again", () => {
    const store = memory();
    saveChoice(store, "light");
    expect(store.values.get(THEME_KEY)).toBe("light");
    saveChoice(store, "system");
    expect(store.values.has(THEME_KEY)).toBe(false);
    expect(resolveTheme(storedTheme(store), true)).toBe("dark");
  });
});

/** Run index.html's inline script against a fake page; the class it leaves. */
async function inlineScript(getItem: () => string | null, systemDark: boolean): Promise<Theme> {
  const html = await Bun.file(new URL("../../index.html", import.meta.url)).text();
  const body = html.match(/<script id="theme">([\s\S]*?)<\/script>/)?.[1];
  if (!body) throw new Error('index.html has no <script id="theme">');
  expect(body).toContain(`"${THEME_KEY}"`);
  const classes = new Set<string>();
  const document = {
    documentElement: {
      classList: {
        toggle: (name: string, on: boolean) => (on ? classes.add(name) : classes.delete(name)),
      },
      style: { colorScheme: "" },
    },
  };
  new Function("localStorage", "matchMedia", "document", body)(
    { getItem },
    () => ({ matches: systemDark }),
    document,
  );
  const theme: Theme = classes.has("dark") ? "dark" : "light";
  expect(document.documentElement.style.colorScheme).toBe(theme);
  return theme;
}

describe("index.html's inline script", () => {
  test("agrees with theme.ts on every stored value and system preference", async () => {
    for (const value of ["light", "dark", null, "purple"]) {
      for (const systemDark of [true, false]) {
        const expected = resolveTheme(storedTheme(storage(value)), systemDark);
        expect(await inlineScript(() => value, systemDark)).toBe(expected);
      }
    }
  });

  test("storage that throws falls back to the system", async () => {
    expect(await inlineScript(throwing.getItem, true)).toBe("dark");
    expect(await inlineScript(throwing.getItem, false)).toBe("light");
  });
});
