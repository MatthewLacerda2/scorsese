// What the landing keeps across the Google sign-in, and what the hand-off
// finds (#1535): taken once, the files with it, and what could not be kept
// named rather than lost.

import { expect, test } from "bun:test";
import { type FileBox, keep, MAX_KEPT_BYTES, take } from "@/prompt/handOff";

/** A box in memory; `refuse` makes it throw on `put`, as a private window's might. */
function memoryBox(refuse = false): FileBox & { held: () => File[] | undefined } {
  let held: File[] | undefined;
  return {
    held: () => held,
    put: async (files) => {
      if (refuse) throw new Error("QuotaExceededError");
      held = files;
    },
    take: async () => {
      const files = held;
      held = undefined;
      return files;
    },
    clear: async () => {
      held = undefined;
    },
  };
}

function memoryStorage(): Storage {
  const map = new Map<string, string>();
  return {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
    removeItem: (key: string) => void map.delete(key),
  } as Storage;
}

const still = new File(["png"], "still.png", { type: "image/png" });
const song = new File(["mp3"], "song.mp3", { type: "audio/mpeg" });

test("the prompt and its files come back once, and a second take finds nothing", async () => {
  const box = memoryBox();
  const storage = memoryStorage();
  await keep("a promo for my bakery", [still, song], box, storage);
  expect(await take(box, storage)).toEqual({
    prompt: "a promo for my bakery",
    files: [still, song],
    missing: [],
  });
  expect(await take(box, storage)).toBeNull();
  expect(box.held()).toBeUndefined();
});

test("files the browser refuses to keep come back as names to attach again", async () => {
  const box = memoryBox(true);
  const storage = memoryStorage();
  await keep("a trailer", [still], box, storage);
  expect(await take(box, storage)).toEqual({
    prompt: "a trailer",
    files: [],
    missing: ["still.png"],
  });
});

test("over 1 GB the files are not kept, and an earlier visit's are cleared", async () => {
  const box = memoryBox();
  const storage = memoryStorage();
  await keep("before", [song], box, storage);
  const huge = { name: "huge.mov", size: MAX_KEPT_BYTES + 1 } as File;
  await keep("a long film", [huge], box, storage);
  expect(box.held()).toBeUndefined();
  expect(await take(box, storage)).toEqual({
    prompt: "a long film",
    files: [],
    missing: ["huge.mov"],
  });
});

test("nothing kept is nothing taken, and a box that cannot open is no files", async () => {
  const storage = memoryStorage();
  expect(await take(memoryBox(), storage)).toBeNull();
  const broken: FileBox = {
    put: () => Promise.reject(new Error("no")),
    take: () => Promise.reject(new Error("no")),
    clear: () => Promise.reject(new Error("no")),
  };
  await keep("words only", [], broken, storage);
  expect(await take(broken, storage)).toEqual({ prompt: "words only", files: [], missing: [] });
});
