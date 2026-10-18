// A tile is two gestures (#1446): a click opens the asset in the side panel,
// a drag places it on a lane. One must never start the other.

import { describe, expect, test } from "bun:test";
import type { DragEvent } from "react";
import type { EditorProject, ToolAnswer } from "@/api";
import { SCHEMA_VERSION } from "@/api";
import { dropOnto } from "../drop";
import type { Edit } from "../project";
import { dropped } from "./dragged";
import { handler, shot, tile } from "./handlers";

describe("an asset tile", () => {
  test("a click opens the asset in the side panel", () => {
    let opened = 0;
    const view = tile(
      shot,
      () => {},
      () => void opened++,
    );
    handler<() => void>(view, (props) => props.draggable === true, "onClick")();
    expect(opened).toBe(1);
  });

  test("dragged onto the timeline still places a clip", async () => {
    const data = new Map<string, string>();
    const transfer = {
      setData: (k: string, v: string) => data.set(k, v),
      getData: (k: string) => data.get(k) ?? "",
    };
    const event = { dataTransfer: transfer } as unknown as DragEvent;
    handler<(event: DragEvent) => void>(
      tile(shot),
      (props) => props.draggable === true,
      "onDragStart",
    )(event);
    const carried = dropped(event);
    expect(carried).toEqual({ from: "project", asset: "beach", kind: "video" });

    const calls: string[] = [];
    const before = {
      revision: 1,
      document: {
        schema_version: SCHEMA_VERSION,
        name: "p",
        timeline_fps: { num: 30, den: 1 },
        assets: [shot],
        tracks: [],
      },
    } as unknown as EditorProject;
    const run = async (call: Edit): Promise<ToolAnswer | null> => {
      calls.push(call.tool);
      const doc = structuredClone(before.document);
      doc.tracks = [{ id: "v1", kind: "video", clips: [] }];
      if (call.tool === "place_clip")
        doc.tracks[0]?.clips.push({ id: "c1", asset: "beach", start: 0, duration: 30 });
      return { said: [], project: { ...before, document: doc } };
    };
    expect(await dropOnto(run, before, carried as NonNullable<typeof carried>, null, 0, 0, 0)).toBe(
      "c1",
    );
    expect(calls).toEqual(["track_new", "place_clip"]);
  });
});
