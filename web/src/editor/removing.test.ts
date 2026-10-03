// A removal's confirm lists the clips that go, and a yes names exactly those.

import { describe, expect, test } from "bun:test";
import type { ProjectDocument } from "@/api";
import type { Edit } from "./project";
import { assetRemoval, confirmThen, showing, trackRemoval } from "./removing";

const clip = (id: string, asset: string) => ({ id, asset, start: 0, duration: 30 });

const document: ProjectDocument = {
  schema_version: 1,
  name: "T",
  timeline_fps: { num: 30, den: 1 },
  assets: [
    { id: "caption", kind: "text", text: "DAWN" },
    { id: "card", kind: "color" },
    {
      id: "inside",
      kind: "group",
      group: { tracks: [{ id: "g1", kind: "video", clips: [clip("nested", "caption")] }] },
    },
  ],
  tracks: [
    { id: "v1", kind: "video", clips: [clip("first", "caption"), clip("other", "card")] },
    { id: "a1", kind: "audio", clips: [] },
  ],
};

describe("removing an asset", () => {
  test("finds the clips showing it, inside groups too", () => {
    expect(showing(document, "caption")).toEqual(["first", "nested"]);
    expect(showing(document, "nobody")).toEqual([]);
  });

  test("asks with those clips named, and sends exactly them", () => {
    const removal = assetRemoval(document, "caption");
    expect(removal.question).toContain("“first”, “nested”");
    expect(removal.edit).toEqual({
      tool: "asset_remove",
      args: { asset: "caption", clips: ["first", "nested"] },
      edit: true,
    });
  });
});

describe("removing a track", () => {
  test("an empty lane asks plainly and names no clips", () => {
    const removal = trackRemoval({ id: "a1", kind: "audio", clips: [] });
    expect(removal.question).toBe("Remove the track “a1”?");
    expect(removal.edit.args).toEqual({ track: "a1", clips: [] });
  });
});

describe("the confirm", () => {
  test("sends nothing on a no, and the edit on a yes", () => {
    const sent: Edit[] = [];
    const run = async (edit: Edit) => sent.push(edit);
    const removal = assetRemoval(document, "card");
    confirmThen(removal, run, () => false);
    expect(sent).toEqual([]);
    confirmThen(removal, run, () => true);
    expect(sent).toEqual([removal.edit]);
  });
});
