// Which theme a page opens in: the stored choice wins, light otherwise, and a storage that throws reads as "not chosen". Both copies of
// that rule are held here — `theme.ts`, and the inline script in `index.html`
// that runs before the bundle — so they cannot drift apart.

import { describe, expect, test } from "bun:test";
import { KEYS } from "@/lib/storage";
import { saveTheme, storedTheme, type Theme } from "@/lib/theme";

const storage = (value: string | null) => ({
  getItem: (key: string) => (key === KEYS.theme ? value : null),
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
  test("a stored choice wins", () => {
    expect(storedTheme(storage("light"))).toBe("light");
    expect(storedTheme(storage("dark"))).toBe("dark");
  });

  test("with nothing stored, or something that is not a theme, the page is light", () => {
    expect(storedTheme(storage(null))).toBe("light");
    expect(storedTheme(storage("purple"))).toBe("light");
  });

  test("storage that throws reads as light, and saving does not throw", () => {
    expect(storedTheme(throwing)).toBe("light");
    expect(storedTheme(undefined)).toBe("light");
    expect(() => saveTheme(throwing, "dark")).not.toThrow();
  });

  test("each choice reads back as itself", () => {
    const store = memory();
    for (const theme of ["dark", "light"] as const) {
      saveTheme(store, theme);
      expect(storedTheme(store)).toBe(theme);
    }
  });
});

/** Run index.html's inline script against a fake page; the class it leaves. */
async function inlineScript(getItem: () => string | null, systemDark = false): Promise<Theme> {
  const html = await Bun.file(new URL("../../index.html", import.meta.url)).text();
  const body = html.match(/<script id="theme">([\s\S]*?)<\/script>/)?.[1];
  if (!body) throw new Error('index.html has no <script id="theme">');
  expect(body).toContain(`"${KEYS.theme}"`);
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
  test("agrees with theme.ts on every stored value, whatever the system prefers", async () => {
    for (const value of ["light", "dark", null, "purple"]) {
      for (const systemDark of [true, false]) {
        expect(await inlineScript(() => value, systemDark)).toBe(storedTheme(storage(value)));
      }
    }
  });

  test("storage that throws is light, even on a dark system", async () => {
    expect(await inlineScript(throwing.getItem, true)).toBe("light");
  });
});
