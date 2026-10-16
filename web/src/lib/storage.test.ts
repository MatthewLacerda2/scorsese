// The one storage module (#1212): every key distinct, a read that cannot be
// made is the fallback, a write that cannot be made is dropped, and a codec
// that formats to `null` removes its key.
import { expect, test } from "bun:test";
import { type Codec, KEYS, oneOf, read, write } from "@/lib/storage";

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

function memory() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
    removeItem: (key: string) => void values.delete(key),
    values,
  };
}

test("no two keys share a name", () => {
  const names = Object.values(KEYS).map((key) => (typeof key === "string" ? key : key(7)));
  expect(new Set(names).size).toBe(names.length);
});

test("the keys keep the names browsers already hold", () => {
  expect(KEYS.language).toBe("scorsese-language");
  expect(KEYS.theme).toBe("scorsese-theme");
  expect(KEYS.panels).toBe("scorsese.panels");
  expect(KEYS.previewQuality).toBe("scorsese.preview-quality");
  expect(KEYS.effort(3)).toBe("scorsese-effort-3");
  expect(KEYS.chatFull(3)).toBe("scorsese-chat-full-3");
  expect(KEYS.shape(3)).toBe("scorsese.shape.3");
});

const colour = oneOf(["red", "blue"], "red");

test("a value reads back; nothing, junk or a throwing storage is the fallback", () => {
  const store = memory();
  expect(read(store, "k", colour)).toBe("red");
  write(store, "k", colour, "blue");
  expect(read(store, "k", colour)).toBe("blue");
  store.setItem("k", "toString");
  expect(read(store, "k", colour)).toBe("red");
  expect(read(undefined, "k", colour)).toBe("red");
  expect(read(throwing, "k", colour)).toBe("red");
  expect(() => write(throwing, "k", colour, "blue")).not.toThrow();
});

test("a parse that throws is the fallback, and `null` from format removes the key", () => {
  const json: Codec<number | null> = {
    fallback: null,
    parse: (raw) => JSON.parse(raw),
    format: (value) => (value === null ? null : JSON.stringify(value)),
  };
  const store = memory();
  store.setItem("n", "{not json");
  expect(read(store, "n", json)).toBeNull();
  write(store, "n", json, 4);
  expect(store.values.get("n")).toBe("4");
  write(store, "n", json, null);
  expect(store.values.has("n")).toBe(false);
});
