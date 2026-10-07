// The panels' sizes (#863): each held inside 0.3×–1.3× of its default, the
// preview's share winning over a panel's most but never its least, and what a
// browser saved read back — with anything unreadable falling to the defaults.

import { describe, expect, test } from "bun:test";
import {
  DEFAULTS,
  fit,
  HANDLE,
  PREVIEW_SHARE,
  resize,
  SIZES_KEY,
  savedSizes,
  saveSizes,
} from "./sizes";

/** A desktop's editor, in rem: room for every panel at its most. */
const desktop = { width: 120, height: 60 };
/** A laptop's: 1366×768 less the browser and the header. */
const laptop = { width: 85, height: 36 };

const memory = () => {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
  };
};
const throwing = {
  getItem: (): string | null => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
};

describe("resizing a panel", () => {
  test("is held inside 0.3× to 1.3× of its default", () => {
    expect(resize(DEFAULTS, "chat", 100, desktop).chat).toBeCloseTo(22 * 1.3);
    expect(resize(DEFAULTS, "chat", 1, desktop).chat).toBeCloseTo(22 * 0.3);
    expect(resize(DEFAULTS, "assets", 17, desktop).assets).toBe(17);
    expect(resize(DEFAULTS, "timeline", 0, null).timeline).toBeCloseTo(15 * 0.3);
  });

  test("leaves the other panels where they are", () => {
    const after = resize(DEFAULTS, "assets", 18, desktop);
    expect(after).toEqual({ ...DEFAULTS, assets: 18 });
  });

  test("the preview's share wins over a panel's most", () => {
    const narrow = { width: 70, height: 30 };
    const assets = resize(DEFAULTS, "assets", 100, narrow).assets;
    expect(assets).toBeCloseTo(narrow.width * (1 - PREVIEW_SHARE) - 2 * HANDLE - DEFAULTS.chat);
    const timeline = resize(DEFAULTS, "timeline", 100, narrow).timeline;
    expect(timeline).toBeCloseTo(narrow.height * (1 - PREVIEW_SHARE) - HANDLE);
  });

  test("but never over its least", () => {
    const tiny = { width: 20, height: 10 };
    expect(resize(DEFAULTS, "chat", 30, tiny).chat).toBeCloseTo(22 * 0.3);
  });

  test("on a laptop, both sidebars at their most still leave the preview its share", () => {
    const wide = resize(resize(DEFAULTS, "assets", 100, laptop), "chat", 100, laptop);
    const preview = laptop.width - wide.assets - wide.chat - 2 * HANDLE;
    expect(preview).toBeGreaterThanOrEqual(laptop.width * PREVIEW_SHARE - 1e-9);
  });
});

describe("fitting the sizes to the editor", () => {
  test("changes nothing where they fit", () => {
    expect(fit(DEFAULTS, desktop)).toEqual(DEFAULTS);
    expect(fit(DEFAULTS, null)).toEqual(DEFAULTS);
  });

  test("a window made smaller takes from the panels, in proportion, down to their least", () => {
    const most = { assets: 19.5, chat: 28.6, timeline: 19.5 };
    const room = { width: 70, height: 30 };
    const fitted = fit(most, room);
    expect(fitted.assets + fitted.chat).toBeCloseTo(room.width * (1 - PREVIEW_SHARE) - 2 * HANDLE);
    expect(fitted.timeline).toBeCloseTo(room.height * (1 - PREVIEW_SHARE) - HANDLE);
    expect(fitted.chat).toBeGreaterThan(fitted.assets);
    const tiny = fit(most, { width: 10, height: 5 });
    expect(tiny).toEqual({ assets: 15 * 0.3, chat: 22 * 0.3, timeline: 15 * 0.3 });
  });
});

describe("the sizes a browser keeps", () => {
  test("a saved size is read back", () => {
    const storage = memory();
    saveSizes(storage, { assets: 12, chat: 25, timeline: 18 });
    expect(savedSizes(storage)).toEqual({ assets: 12, chat: 25, timeline: 18 });
  });

  test("nothing saved, a stranger's value or a throwing storage reads as the defaults", () => {
    expect(savedSizes(memory())).toEqual(DEFAULTS);
    expect(savedSizes(undefined)).toEqual(DEFAULTS);
    expect(savedSizes(throwing)).toEqual(DEFAULTS);
    const junk = memory();
    junk.setItem(SIZES_KEY, "{not json");
    expect(savedSizes(junk)).toEqual(DEFAULTS);
    junk.setItem(SIZES_KEY, JSON.stringify({ assets: "wide", chat: 24 }));
    expect(savedSizes(junk)).toEqual({ ...DEFAULTS, chat: 24 });
    expect(() => saveSizes(throwing, DEFAULTS)).not.toThrow();
  });

  test("a saved size outside its range is held inside it", () => {
    const storage = memory();
    storage.setItem(SIZES_KEY, JSON.stringify({ assets: 1000, chat: 0, timeline: 15 }));
    expect(savedSizes(storage)).toEqual({ assets: 15 * 1.3, chat: 22 * 0.3, timeline: 15 });
  });
});
