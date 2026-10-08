// The timeline under a pointer (#898): a clip dragged along its lane, or by an
// edge, becomes the one `clip_move` the drag means; a press on the ruler
// scrubs; and a press on the panel handle above it does not (#869). Mounted in
// happy-dom (src/test/dom.ts), which lays nothing out, so a clip is given the
// box the browser would have drawn it in.

import { describe, expect, mock, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ProjectDocument } from "@/api";
import { Handle } from "../panels/Handle";
import type { Release } from "./drag";
import { Timeline } from "./Timeline";
import { START_ZOOM } from "./zoom";

const fps = { num: 30, den: 1 };
/** One clip, two seconds long, starting at two seconds: 80px wide at 80px. */
const document: ProjectDocument = {
  schema_version: 1,
  name: "p",
  timeline_fps: fps,
  assets: [{ id: "a", kind: "text", text: "Title" }],
  tracks: [
    { id: "v1", kind: "video", clips: [{ id: "c1", asset: "a", start: 60, duration: 60 }] },
    { id: "v2", kind: "video", clips: [] },
  ],
};
const px = (seconds: number) => seconds * START_ZOOM;

function mount() {
  const released = mock(async (_call: Release) => {});
  const seeks = mock((_frame: number) => {});
  const none = () => {};
  render(
    <div>
      <Handle grows="up" size={15} label="timeline" onSize={none} onReset={none} />
      <Timeline
        document={document}
        playhead={0}
        onSeek={seeks}
        selected={[]}
        onSelect={none}
        onDeselect={none}
        onRelease={released}
        onDrop={none}
        onRemoveTrack={none}
      />
    </div>,
  );
  const clip = screen.getByTitle(/c1/);
  const left = 200;
  clip.getBoundingClientRect = () =>
    ({ left, top: 0, width: px(2), height: 40, right: left + px(2), bottom: 40 }) as DOMRect;
  // happy-dom has no hit-testing: a lane is under a point 40px or less from
  // the top, and the second lane below that, as the browser would draw them.
  const lane = (id: string) => window.document.querySelector(`[data-track="${id}"]`) as Element;
  window.document.elementsFromPoint = (_x: number, y: number) => [lane(y <= 40 ? "v1" : "v2")];
  return { released, seeks, clip, left };
}

/** A press at `from` on `element`, moved to `to` and let go there, in client pixels. */
function drag(element: Element, from: number, to: number, down = 0) {
  fireEvent.pointerDown(element, { button: 0, pointerId: 1, clientX: from, clientY: 20 });
  fireEvent.pointerMove(element, { pointerId: 1, clientX: to, clientY: 20 + down });
  fireEvent.pointerUp(element, { pointerId: 1, clientX: to, clientY: 20 + down });
}

describe("a clip dragged on the timeline", () => {
  test("by its body, becomes one clip_move to the new start", () => {
    const { released, clip, left } = mount();
    drag(clip, left + px(1), left + px(2));
    expect(released).toHaveBeenCalledTimes(1);
    expect(released.mock.calls[0]?.[0]).toEqual({
      tool: "clip_move",
      args: { clip: "c1", start_seconds: 3 },
    });
  });

  test("onto the lane below, becomes one clip_move carrying the lane and the start", () => {
    const { released, clip, left } = mount();
    drag(clip, left + px(1), left + px(1.5), 48);
    expect(released.mock.calls[0]?.[0]).toEqual({
      tool: "clip_move",
      args: { clip: "c1", track: "v2", start_seconds: 2.5 },
    });
  });

  test("by its right edge, becomes one clip_move to the new length", () => {
    const { released, clip, left } = mount();
    drag(clip, left + px(2) - 2, left + px(1.5) - 2);
    expect(released.mock.calls[0]?.[0]).toEqual({
      tool: "clip_move",
      args: { clip: "c1", duration_seconds: 1.5 },
    });
  });

  test("let go where it began, asks the server for nothing", () => {
    const { released, clip, left } = mount();
    drag(clip, left + px(1), left + px(1));
    expect(released).not.toHaveBeenCalled();
  });
});

describe("the timeline's panel handle", () => {
  test("a press on the ruler scrubs, and a drag on the handle above it does not", () => {
    const { seeks } = mount();
    const ruler = screen.getByText("0s").parentElement as HTMLElement;
    fireEvent.pointerDown(ruler, { button: 0, pointerId: 2, clientX: 40, clientY: 5 });
    expect(seeks).toHaveBeenCalledTimes(1);
    seeks.mockClear();
    drag(screen.getByRole("separator", { name: "timeline" }), 40, 120);
    expect(seeks).not.toHaveBeenCalled();
  });
});
