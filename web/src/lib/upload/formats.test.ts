// The drop rules: which files a tray takes, by the library's own list, and
// which drags are files at all.

import { expect, test } from "bun:test";
import type { Formats } from "@/api";
import { accept, carriesFiles, kindOf, sort } from "./formats";

const formats: Formats = {
  video: ["mp4", "mov"],
  image: ["png", "jpg"],
  audio: ["mp3"],
  html: ["html"],
  text: ["txt", "md"],
};

const file = (name: string) => new File(["x"], name);

test("a file's kind is its extension's, in any case", () => {
  expect(kindOf("Trip.MOV", formats)).toBe("video");
  expect(kindOf("notes.md", formats)).toBe("text");
  expect(kindOf("tune.mid", formats)).toBeNull();
  expect(kindOf("README", formats)).toBeNull();
});

test("files the library refuses are named, the rest kept in order", () => {
  const { taken, refused } = sort(
    [file("a.png"), file("b.mid"), file("c.mp3"), file("d")],
    formats,
  );
  expect(taken.map(({ file: f, kind }) => [f.name, kind])).toEqual([
    ["a.png", "image"],
    ["c.mp3", "audio"],
  ]);
  expect(refused).toEqual(["b.mid", "d"]);
});

test("before the list has loaded, nothing is refused: the server still is the backstop", () => {
  const { taken, refused } = sort([file("b.mid")], undefined);
  expect(taken.map(({ kind }) => kind)).toEqual([null]);
  expect(refused).toEqual([]);
});

test("the picker's hint is every extension the library takes", () => {
  expect(accept(formats)).toBe(".mp4,.mov,.png,.jpg,.mp3,.html,.txt,.md");
  expect(accept(undefined)).toBeUndefined();
});

test("only a drag carrying files is an attachment, never the editor's own", () => {
  expect(carriesFiles(["Files"])).toBe(true);
  expect(carriesFiles(["application/x-scorsese-asset"])).toBe(false);
  expect(carriesFiles(["text/plain", "text/uri-list"])).toBe(false);
});
