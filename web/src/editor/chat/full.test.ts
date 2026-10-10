// Full mode is remembered per project, and nothing stored — or a storage that
// fails — opens the editor (#1027).

import { expect, test } from "bun:test";
import { fullKey, saveFull, storedFull } from "./full";

function memory(): Storage {
  const held = new Map<string, string>();
  return {
    getItem: (key: string) => held.get(key) ?? null,
    setItem: (key: string, value: string) => void held.set(key, value),
  } as Storage;
}

test("nothing remembered is the editor", () => {
  expect(storedFull(memory(), 7)).toBe(false);
  expect(storedFull(undefined, 7)).toBe(false);
});

test("the mode is remembered for its own project only, both ways", () => {
  const storage = memory();
  saveFull(storage, 7, true);
  expect(storedFull(storage, 7)).toBe(true);
  expect(storedFull(storage, 8)).toBe(false);
  saveFull(storage, 7, false);
  expect(storedFull(storage, 7)).toBe(false);
});

test("a stored value that is not the mode, or a storage that throws, is the editor", () => {
  const storage = memory();
  storage.setItem(fullKey(7), "yes");
  expect(storedFull(storage, 7)).toBe(false);
  const broken = {
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("denied");
    },
  } as unknown as Storage;
  expect(storedFull(broken, 7)).toBe(false);
  expect(() => saveFull(broken, 7, true)).not.toThrow();
});
