import { expect, test } from "bun:test";
import { withoutExtension } from "./FileTile";

test("a tile shows its name without the extension", () => {
  expect(withoutExtension("beach.mp4")).toBe("beach");
  expect(withoutExtension("take.2.final.wav")).toBe("take.2.final");
});

test("a name with no extension stays whole", () => {
  expect(withoutExtension("notes")).toBe("notes");
  expect(withoutExtension(".hidden")).toBe(".hidden");
});
