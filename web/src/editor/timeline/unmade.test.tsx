// A clip nobody has made yet is hatched, as the desktop app draws one; a made
// clip is a solid block. Rendered through `react-dom/server`, so no DOM.

import { describe, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { Lane } from "./Lane";
import { isMade } from "./made";

const shot = (id: string, extra: Partial<DocumentAsset>): DocumentAsset => ({
  id,
  kind: "generated_video",
  prompt: "a lighthouse at dusk",
  ...extra,
});

function lane(asset: DocumentAsset): string {
  const track = {
    id: "v1",
    kind: "video" as const,
    clips: [{ id: "c1", asset: asset.id, start: 0, duration: 30 }],
  };
  const document: ProjectDocument = {
    schema_version: 1,
    name: "p",
    timeline_fps: { num: 30, den: 1 },
    assets: [asset],
    tracks: [track],
  };
  const none = () => {};
  return renderToString(
    <Lane
      track={track}
      document={document}
      width={400}
      zoom={{ pxPerSecond: 80 }}
      playhead={0}
      selected={[]}
      onSelect={none}
      onRelease={async () => {}}
      onEmptyClick={none}
      onDragOver={none}
      onDrop={none}
    />,
  );
}

describe("the lane", () => {
  test("hatches a sketch and says so", () => {
    const html = lane(shot("s", { state: "sketch" }));
    expect(html).toContain('data-made="false"');
    expect(html).toContain("mask-image");
    expect(html).toContain(">sketch<");
  });

  test("hatches a stale shot, though its old file is still there", () => {
    const html = lane(shot("s", { state: "stale", path: "generated/s.mp4" }));
    expect(html).toContain('data-made="false"');
    expect(html).toContain(">stale<");
  });

  test("draws a generated shot solid, with no state on it", () => {
    const html = lane(shot("g", { state: "generated", path: "generated/g.mp4" }));
    expect(html).toContain('data-made="true"');
    expect(html).not.toContain("mask-image");
    expect(html).not.toContain(">generated<");
  });
});

describe("made", () => {
  test("follows the app's rule", () => {
    expect(isMade(undefined)).toBe(false);
    expect(isMade(shot("q", { state: "queued" }))).toBe(false);
    expect(isMade(shot("g", { state: "generated" }))).toBe(false);
    expect(isMade({ id: "v", kind: "video", path: "assets/v.mp4" })).toBe(true);
    expect(isMade({ id: "t", kind: "text", text: "Hello" })).toBe(true);
  });
});
