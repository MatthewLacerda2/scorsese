// Every catalogue holds every message English has — no silent fallbacks
// (#704). The types already refuse a missing key; this holds the same at run
// time, and checks what types cannot: no message is left empty, and a message
// that takes a value takes the same number of them in every language.

import { describe, expect, test } from "bun:test";
import { CATALOGUES, messagesFor, withFallback } from "@/i18n/catalogue";
import { en } from "@/i18n/en";
import { LANGUAGES } from "@/i18n/language";

/** Every message as `path → value`, nested objects walked. */
function flatten(node: unknown, prefix = ""): Map<string, unknown> {
  const out = new Map<string, unknown>();
  for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "object" && value !== null) {
      for (const entry of flatten(value, path)) out.set(...entry);
    } else out.set(path, value);
  }
  return out;
}

const english = flatten(en);

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  const messages = flatten(CATALOGUES[language]);

  test("has exactly English's keys", () => {
    expect([...messages.keys()].sort()).toEqual([...english.keys()].sort());
  });

  test("leaves no message empty, and each takes the arguments English does", () => {
    for (const [path, value] of messages) {
      const source = english.get(path);
      expect({ path, type: typeof value }).toEqual({ path, type: typeof source });
      if (typeof value === "string") expect({ path, value: value.trim() }).not.toEqual({ path, value: "" });
      if (typeof value === "function" && typeof source === "function") {
        expect({ path, arity: value.length }).toEqual({ path, arity: source.length });
      }
    }
  });
});

test("a message a catalogue lacks reads in English", () => {
  const partial = { common: { close: "Fechar" } } as unknown as typeof en;
  const merged = withFallback(partial, en);
  expect(merged.common.close).toBe("Fechar");
  expect(merged.common.cancel).toBe(en.common.cancel);
});

test("each language reads its own words", () => {
  expect(messagesFor("en").common.logOut).toBe("Log out");
  expect(messagesFor("pt-BR").common.logOut).toBe("Sair");
  expect(messagesFor("es").common.logOut).toBe("Cerrar sesión");
});
