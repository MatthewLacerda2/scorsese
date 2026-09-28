// One track's lane and the clips on it, and the drag that moves or trims one.
//
// A clip's body may be dragged onto another lane as well as along its own —
// the desktop app's drag does the same — and letting go there is one
// `clip_move` with the new start. An edge never changes lane: the edge being
// pulled belongs to the clip where it is. Whether the lane will take the clip
// (picture onto a sound track, a clip already there) is the server's answer.

import { type DragEvent, type PointerEvent, useState } from "react";
import type { Clip, ProjectDocument, Track } from "@/api";
import { kindColor } from "../assets/kinds";
import { adds } from "../selection";
import {
  type Handle,
  type Limits,
  limitsOf,
  type Release,
  type Shape,
  SNAP_PX,
  shapeOf,
  snapped,
  targets,
  toolCall,
} from "./drag";
import { framesToPx, pxSpan, type Zoom } from "./time";

/** How close to a clip's edge, in pixels, a press takes the edge rather than the clip. */
const EDGE_PX = 6;

interface Props {
  track: Track;
  document: ProjectDocument;
  width: number;
  zoom: Zoom;
  playhead: number;
  selected: string[];
  /** A clip clicked: `adding` when a modifier asks to add it to the selection. */
  onSelect: (clip: string, adding: boolean) => void;
  /** A drag let go: the one tool call it becomes. */
  onRelease: (call: Release) => Promise<unknown>;
  onEmptyClick: (event: PointerEvent, lane: Element) => void;
  onDragOver: (event: DragEvent) => void;
  onDrop: (event: DragEvent, lane: Element) => void;
}

/** A drag in progress: what was taken hold of, where from, and what it proposes now. */
interface Held {
  clip: Clip;
  handle: Handle;
  from: number;
  limits: Limits;
  at: number[];
  shape: Shape;
  /** Where the pointer went down, vertically, and how far it has gone since —
   * a body drag follows it off the lane. */
  y: number;
  dy: number;
}

/** The lane under a point: its track id, from the `data-track` it carries.
 * Every element there rather than the topmost, since the clip being dragged is
 * itself under the pointer. */
function laneAt(x: number, y: number): string | null {
  for (const found of window.document.elementsFromPoint(x, y)) {
    if (found instanceof HTMLElement && found.dataset.track) return found.dataset.track;
  }
  return null;
}

export function Lane(props: Props) {
  const { track, document, zoom } = props;
  const fps = document.timeline_fps;
  const [held, setHeld] = useState<Held | null>(null);
  const [landing, setLanding] = useState<{ clip: string; shape: Shape; dy: number } | null>(null);
  const assets = document.assets ?? [];

  const grab = (event: PointerEvent<HTMLElement>, clip: Clip) => {
    event.stopPropagation();
    props.onSelect(clip.id, adds(event));
    const box = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - box.left;
    const handle: Handle = x < EDGE_PX ? "left" : x > box.width - EDGE_PX ? "right" : "body";
    const asset = assets.find((found) => found.id === clip.asset);
    event.currentTarget.setPointerCapture(event.pointerId);
    setHeld({
      clip,
      handle,
      from: event.clientX,
      limits: limitsOf(clip, asset, fps),
      at: targets(document, props.playhead, clip.id),
      shape: shapeOf(clip),
      y: event.clientY,
      dy: 0,
    });
  };
  // Where the pointer is now proposes the shape — worked out from the event
  // itself on release too, since the last move may not have been drawn yet.
  const proposal = (event: PointerEvent, from: Held) => {
    const delta = pxSpan(event.clientX - from.from, zoom, fps);
    const reach = pxSpan(SNAP_PX, zoom, fps);
    return snapped(from.clip, from.handle, delta, from.limits, from.at, reach);
  };
  const follow = (event: PointerEvent) => {
    if (!held) return;
    const dy = held.handle === "body" ? event.clientY - held.y : 0;
    setHeld({ ...held, shape: proposal(event, held), dy });
  };
  const release = (event: PointerEvent) => {
    if (!held) return;
    const shape = proposal(event, held);
    const onto =
      held.handle === "body" ? (laneAt(event.clientX, event.clientY) ?? track.id) : track.id;
    const call = toolCall(held.clip, track.id, onto, shape, fps);
    setHeld(null);
    if (!call) return;
    // Drawn where it was let go until the server answers: moved there, or
    // refused and springing back.
    setLanding({ clip: held.clip.id, shape, dy: held.dy });
    void props.onRelease(call).finally(() => setLanding(null));
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a lane is a pointer surface — seeking and dropping — whose keyboard equivalents are the transport and the inspector
    <div
      className="relative h-12 shrink-0"
      data-track={track.id}
      style={{ width: props.width }}
      onPointerDown={(event) => props.onEmptyClick(event, event.currentTarget)}
      onDragOver={props.onDragOver}
      onDrop={(event) => props.onDrop(event, event.currentTarget)}
    >
      {track.clips.map((clip) => {
        const shape =
          held?.clip.id === clip.id
            ? held.shape
            : landing?.clip === clip.id
              ? landing.shape
              : shapeOf(clip);
        const dy = held?.clip.id === clip.id ? held.dy : landing?.clip === clip.id ? landing.dy : 0;
        const asset = assets.find((found) => found.id === clip.asset);
        const chosen = props.selected.includes(clip.id);
        return (
          <button
            type="button"
            key={clip.id}
            title={`${clip.id} — drag to move (onto another lane too), drag an edge to trim, Shift-click to select several, Delete to remove`}
            className={`absolute top-1 bottom-1 cursor-grab touch-none overflow-hidden rounded-sm px-1.5 text-[11px] text-white select-none ${kindColor(asset?.kind)} ${chosen ? "ring-2 ring-foreground" : "opacity-90"} ${held?.clip.id === clip.id ? "z-10 cursor-grabbing shadow-lg" : ""}`}
            style={{
              left: framesToPx(shape.start, zoom, fps),
              width: Math.max(3, framesToPx(shape.duration, zoom, fps)),
              transform: dy ? `translateY(${dy}px)` : undefined,
            }}
            onPointerDown={(event) => grab(event, clip)}
            onPointerMove={follow}
            onPointerUp={release}
            onPointerCancel={() => setHeld(null)}
            onKeyDown={(event) => {
              if (event.key === "Enter") props.onSelect(clip.id, adds(event));
            }}
          >
            <span className="pointer-events-none block truncate leading-10">
              {asset?.text ?? clip.asset}
            </span>
            <span
              className="absolute inset-y-0 left-0 w-1.5 cursor-ew-resize bg-black/20"
              aria-hidden
            />
            <span
              className="absolute inset-y-0 right-0 w-1.5 cursor-ew-resize bg-black/20"
              aria-hidden
            />
          </button>
        );
      })}
    </div>
  );
}
