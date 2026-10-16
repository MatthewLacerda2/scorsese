// The level is remembered per project, and a storage that fails reads as
// Thorough (#769).

import { expect, test } from "bun:test";
import { KEYS } from "@/lib/storage";
import { DEFAULT_EFFORT, saveEffort, storedEffort } from "./effort";

function memory(): Storage {
  const held = new Map<string, string>();
  return {
    getItem: (key: string) => held.get(key) ?? null,
    setItem: (key: string, value: string) => void held.set(key, value),
  } as Storage;
}

test("nothing chosen is Thorough", () => {
  expect(DEFAULT_EFFORT).toBe("high");
  expect(storedEffort(memory(), 7)).toBe("high");
  expect(storedEffort(undefined, 7)).toBe("high");
});

test("a choice is remembered for its own project only", () => {
  const storage = memory();
  saveEffort(storage, 7, "low");
  expect(storedEffort(storage, 7)).toBe("low");
  expect(storedEffort(storage, 8)).toBe("high");
});

test("a stored value that is not a level, or a storage that throws, is Thorough", () => {
  const storage = memory();
  storage.setItem(KEYS.effort(7), "max");
  expect(storedEffort(storage, 7)).toBe("high");
  const broken = {
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("denied");
    },
  } as unknown as Storage;
  expect(storedEffort(broken, 7)).toBe("high");
  expect(() => saveEffort(broken, 7, "low")).not.toThrow();
});
