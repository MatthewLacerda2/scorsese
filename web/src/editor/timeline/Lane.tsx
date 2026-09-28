// One track's lane and the clips on it, and the drag that moves or trims one.
//
// A clip moves along its own lane only: which track a clip sits on decides
// what is drawn over what, so moving it to another is a sentence to the
// assistant (`trim_clip` keeps a clip on its track, as its description says).

import { type DragEvent, type PointerEvent, useState } from "react";
import type { Clip, ProjectDocument, Track } from "@/api";
import { kindColor } from "../assets/kinds";
import {
  type Handle,
  type Limits,
  limitsOf,
  type Shape,
  SNAP_PX,
  shapeOf,
  snapped,
  targets,
  trimArguments,
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
  selected: string | null;
  onSelect: (clip: string) => void;
  onTrim: (args: Record<string, unknown>) => Promise<unknown>;
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
}

export function Lane(props: Props) {
  const { track, document, zoom } = props;
  const fps = document.timeline_fps;
  const [held, setHeld] = useState<Held | null>(null);
  const [landing, setLanding] = useState<{ clip: string; shape: Shape } | null>(null);
  const assets = document.assets ?? [];

  const grab = (event: PointerEvent<HTMLElement>, clip: Clip) => {
    event.stopPropagation();
    props.onSelect(clip.id);
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
    if (held) setHeld({ ...held, shape: proposal(event, held) });
  };
  const release = (event: PointerEvent) => {
    if (!held) return;
    const shape = proposal(event, held);
    const args = trimArguments(held.clip, shape, fps);
    setHeld(null);
    if (!args) return;
    // Drawn where it was let go until the server answers: moved there, or
    // refused and springing back.
    setLanding({ clip: held.clip.id, shape });
    void props.onTrim(args).finally(() => setLanding(null));
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a lane is a pointer surface — seeking and dropping — whose keyboard equivalents are the transport and the inspector
    <div
      className="relative h-12 shrink-0"
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
        const asset = assets.find((found) => found.id === clip.asset);
        const chosen = props.selected === clip.id;
        return (
          <button
            type="button"
            key={clip.id}
            title={`${clip.id} — drag to move, drag an edge to trim`}
            className={`absolute top-1 bottom-1 cursor-grab touch-none overflow-hidden rounded-sm px-1.5 text-[11px] text-white select-none ${kindColor(asset?.kind)} ${chosen ? "ring-2 ring-foreground" : "opacity-90"} ${held?.clip.id === clip.id ? "cursor-grabbing shadow-lg" : ""}`}
            style={{
              left: framesToPx(shape.start, zoom, fps),
              width: Math.max(3, framesToPx(shape.duration, zoom, fps)),
            }}
            onPointerDown={(event) => grab(event, clip)}
            onPointerMove={follow}
            onPointerUp={release}
            onPointerCancel={() => setHeld(null)}
            onKeyDown={(event) => {
              if (event.key === "Enter") props.onSelect(clip.id);
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
