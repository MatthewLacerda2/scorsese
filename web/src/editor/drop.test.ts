// A drop never needs a track made first: on an empty timeline, below the last
// lane and on a lane of the other kind it makes one of the asset's kind and
// places the clip there; on a lane that takes it, it places it there.

import { describe, expect, test } from "bun:test";
import type { EditorProject, ProjectDocument, ToolAnswer, Track } from "@/api";
import { dropOnto } from "./drop";
import type { Edit } from "./project";

const THIRTY = { num: 30, den: 1 };
const assets = [
  { id: "shot", kind: "video" as const, media: { duration_seconds: 4 } },
  { id: "music", kind: "audio" as const, media: { duration_seconds: 2 } },
];
const project = (tracks: Track[]): EditorProject =>
  ({
    revision: 1,
    document: { schema_version: 47, name: "t", timeline_fps: THIRTY, assets, tracks },
  }) as unknown as EditorProject;

/** A server that does what `track_new` and `place_clip` do, and remembers the calls. */
function server(start: EditorProject) {
  let now = structuredClone(start);
  const calls: Edit[] = [];
  const run = async (edit: Edit): Promise<ToolAnswer | null> => {
    calls.push(edit);
    const doc: ProjectDocument = structuredClone(now.document);
    const tracks = doc.tracks ?? [];
    if (edit.tool === "track_new") {
      const kind = edit.args.kind as Track["kind"];
      const n = tracks.filter((track) => track.kind === kind).length + 1;
      tracks.push({ id: `${kind[0]}${n}`, kind, clips: [] });
    } else {
      const track = tracks.find((found) => found.id === edit.args.track);
      track?.clips.push({
        id: String(edit.args.asset),
        asset: String(edit.args.asset),
        start: 0,
        duration: 30,
      });
    }
    now = { ...now, document: { ...doc, tracks } };
    return { said: [], project: now };
  };
  return { run, calls };
}

const drop = (start: EditorProject, asset: "shot" | "music", track: Track | null) => {
  const fake = server(start);
  const kind = asset === "shot" ? "video" : "audio";
  const added = dropOnto(fake.run, start, { from: "project", asset, kind }, track, 0, 0, 0);
  return { fake, added };
};

describe("a drop", () => {
  test("on an empty timeline makes the first lane, of the asset's kind", async () => {
    const { fake, added } = drop(project([]), "music", null);
    expect(await added).toBe("music");
    expect(fake.calls.map((call) => call.tool)).toEqual(["track_new", "place_clip"]);
    expect(fake.calls[0]?.args).toEqual({ kind: "audio" });
    expect(fake.calls[1]?.args.track).toBe("a1");
  });

  test("of a sound on a picture lane makes an audio lane for it", async () => {
    const video: Track = { id: "v1", kind: "video", clips: [] };
    const { fake, added } = drop(project([video]), "music", video);
    expect(await added).toBe("music");
    expect(fake.calls[0]?.args).toEqual({ kind: "audio" });
    expect(fake.calls[1]?.args.track).toBe("a1");
  });

  test("below the last lane makes another lane even when one would take it", async () => {
    const video: Track = { id: "v1", kind: "video", clips: [] };
    const { fake, added } = drop(project([video]), "shot", null);
    expect(await added).toBe("shot");
    expect(fake.calls[0]?.args).toEqual({ kind: "video" });
    expect(fake.calls[1]?.args.track).toBe("v2");
  });

  test("on a lane that takes it places it there and makes nothing", async () => {
    const video: Track = { id: "v1", kind: "video", clips: [] };
    const { fake, added } = drop(project([video]), "shot", video);
    expect(await added).toBe("shot");
    expect(fake.calls.map((call) => call.tool)).toEqual(["place_clip"]);
    expect(fake.calls[0]?.args.track).toBe("v1");
  });

  test("whose new lane is refused places nothing", async () => {
    const refusing = async (): Promise<ToolAnswer | null> => null;
    const start = project([]);
    const dragged = { from: "project" as const, asset: "shot", kind: "video" };
    expect(await dropOnto(refusing, start, dragged, null, 0, 0, 0)).toBeNull();
  });
});
