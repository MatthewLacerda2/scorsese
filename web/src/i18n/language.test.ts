// Which language a page opens in: the stored choice wins, the browser's
// preferred languages decide otherwise — the first we speak, Brazilian
// Portuguese when none is (#770) — and a storage that throws reads as "not
// chosen".

import { expect, test } from "bun:test";
import {
  browserLanguage,
  LANGUAGE_KEY,
  resolveLanguage,
  saveLanguage,
  storedLanguage,
} from "@/i18n/language";

const storage = (value: string | null) => ({
  getItem: (key: string) => (key === LANGUAGE_KEY ? value : null),
});
const throwing = {
  getItem: (): string | null => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
};

test("each browser language picks the nearest one we speak", () => {
  expect(browserLanguage(["pt-BR"])).toBe("pt-BR");
  expect(browserLanguage(["pt-PT"])).toBe("pt-BR");
  expect(browserLanguage(["es-419"])).toBe("es");
  expect(browserLanguage(["ES"])).toBe("es");
  expect(browserLanguage(["en-GB"])).toBe("en");
});

test("the first preferred language we speak wins, in the browser's order", () => {
  expect(browserLanguage(["fr", "es"])).toBe("es");
  expect(browserLanguage(["de-DE", "en-US", "pt-BR"])).toBe("en");
  expect(browserLanguage(["es", "en"])).toBe("es");
});

test("no language we speak, or none at all, opens in Brazilian Portuguese", () => {
  expect(browserLanguage(["fr"])).toBe("pt-BR");
  expect(browserLanguage(["fr-FR", "de"])).toBe("pt-BR");
  expect(browserLanguage([])).toBe("pt-BR");
  expect(browserLanguage(undefined)).toBe("pt-BR");
});

test("a stored choice wins over every browser language", () => {
  expect(resolveLanguage(storedLanguage(storage("es")), ["pt-BR"])).toBe("es");
  expect(resolveLanguage(storedLanguage(storage("en")), ["fr", "es"])).toBe("en");
  expect(resolveLanguage(storedLanguage(storage("es")), [])).toBe("es");
  expect(resolveLanguage(storedLanguage(storage(null)), ["fr", "es"])).toBe("es");
  expect(resolveLanguage(storedLanguage(storage(null)), ["fr"])).toBe("pt-BR");
});

test("a stored value that is not a language, or a storage that throws, is no choice", () => {
  expect(storedLanguage(storage("klingon"))).toBeNull();
  expect(storedLanguage(throwing)).toBeNull();
  expect(storedLanguage(undefined)).toBeNull();
  expect(() => saveLanguage(throwing, "es")).not.toThrow();
});

test("a saved choice reads back", () => {
  const values = new Map<string, string>();
  const memory = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
  };
  saveLanguage(memory, "pt-BR");
  expect(storedLanguage(memory)).toBe("pt-BR");
});
