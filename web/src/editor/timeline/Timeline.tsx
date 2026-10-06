// The timeline: a ruler to scrub on, one lane per track, and the clips on
// them — dragged to move, dragged by an edge to trim, and dropped onto from the
// assets panel. The desktop app's `app/src/timeline/`, in the browser.
//
// A drag is drawn where the pointer proposes, snapped (drag.ts), and becomes
// one `clip_move` — with the new lane, let go over another — when the
// pointer lets go; never before, since every edit is a round trip to the
// server's tools. A refused one springs back: the page only ever draws the
// document the server holds. A lane header's bin removes the lane, after a
// confirm listing the clips on it (#396).
//
// There are no buttons (#765): a pinch or Ctrl+wheel zooms around the pointer
// (zoom.ts), and the space below the last lane is where a drop makes a new
// one — as does a drop on a lane of the other kind (drop.ts).

import { Trash2Icon } from "lucide-react";
import { type DragEvent, type PointerEvent, useLayoutEffect, useRef, useState } from "react";
import type { Fps, ProjectDocument, Track } from "@/api";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { type Dragged, dropped, isOurs } from "../assets/dragged";
import { type Release, SNAP_PX } from "./drag";
import { Lane } from "./Lane";
import { NewLane, useCarrying } from "./NewLane";
import { Ruler } from "./Ruler";
import { framesToPx, pxSpan, pxToFrames, toFrames, type Zoom } from "./time";
import { keptUnder, START_ZOOM, zoomed } from "./zoom";

/** How wide the lane headers are, in pixels. */
const HEADER = 112;

export interface TimelineProps {
  document: ProjectDocument;
  playhead: number;
  onSeek: (frame: number) => void;
  selected: string[];
  /** A clip clicked, `adding` it to the selection when a modifier was held. */
  onSelect: (clip: string, adding: boolean) => void;
  /** An empty stretch of a lane clicked: nothing is selected. */
  onDeselect: () => void;
  /** A drag let go: one `clip_move`, along a lane or onto another. */
  onRelease: (call: Release) => Promise<unknown>;
  /** Something dropped on `track` — or, `null`, below the last lane — at
   * frame `pointed`, before snapping. */
  onDrop: (dragged: Dragged, track: Track | null, pointed: number, reach: number) => void;
  /** A lane header's bin pressed: confirm, then remove `track` and its clips. */
  onRemoveTrack: (track: Track) => void;
}

export function Timeline(props: TimelineProps) {
  const { document, playhead, onSeek } = props;
  const t = useT().editor;
  const fps: Fps = document.timeline_fps;
  const [zoom, setZoom] = useState<Zoom>({ pxPerSecond: START_ZOOM });
  const scroller = useRef<HTMLDivElement>(null);
  const carrying = useCarrying();
  // The scroll a zoom wants, applied once the timeline has been drawn at the
  // new width — before then the browser would clamp it to the old one.
  const pendingScroll = useRef<number | null>(null);
  useLayoutEffect(() => {
    if (pendingScroll.current === null || !scroller.current) return;
    scroller.current.scrollLeft = pendingScroll.current;
    pendingScroll.current = null;
  });
  // A native listener, because React's wheel handler is passive and only an
  // active one may stop the browser zooming the whole page instead.
  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return;
      event.preventDefault();
      const pointer = event.clientX - element.getBoundingClientRect().left - HEADER;
      setZoom((from) => {
        const to = zoomed(from, event.deltaY, event.deltaMode);
        pendingScroll.current = keptUnder(Math.max(0, pointer), element.scrollLeft, from, to);
        return to;
      });
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, []);
  const tracks = document.tracks ?? [];
  const end = Math.max(
    0,
    ...tracks.flatMap((track) => track.clips.map((c) => c.start + c.duration)),
  );
  // Room after the last clip to drop the next one, and never less than a minute.
  const width = framesToPx(Math.max(end + toFrames(10, fps), toFrames(60, fps)), zoom, fps);
  const frameAt = (event: PointerEvent | DragEvent, lane: Element) =>
    pxToFrames(event.clientX - lane.getBoundingClientRect().left, zoom, fps);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div ref={scroller} className="relative min-h-0 flex-1 overflow-auto">
        <div className="relative flex min-h-full flex-col" style={{ width: width + HEADER }}>
          <div className="sticky top-0 z-20 flex bg-background">
            <div
              className="sticky left-0 z-30 shrink-0 border-r bg-background"
              style={{ width: HEADER }}
            />
            <Ruler
              width={width}
              zoom={zoom}
              fps={fps}
              onScrub={(event, ruler) => onSeek(frameAt(event, ruler))}
            />
          </div>
          {tracks.map((track) => (
            <div key={track.id} className="flex border-b">
              <div
                className="sticky left-0 z-10 flex shrink-0 items-center border-r bg-background pl-2"
                style={{ width: HEADER }}
              >
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-xs font-medium">{track.name ?? track.id}</span>
                  <span className="text-[10px] text-muted-foreground">
                    {t.trackKinds[track.kind] ?? track.kind}
                  </span>
                </div>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={t.timeline.removeTrack(track.name ?? track.id)}
                  title={t.timeline.removeTrackTitle}
                  onClick={() => props.onRemoveTrack(track)}
                >
                  <Trash2Icon />
                </Button>
              </div>
              <Lane
                track={track}
                document={document}
                width={width}
                zoom={zoom}
                playhead={playhead}
                selected={props.selected}
                onSelect={props.onSelect}
                onRelease={props.onRelease}
                onEmptyClick={(event, lane) => {
                  props.onDeselect();
                  onSeek(frameAt(event, lane));
                }}
                onDragOver={(event) => {
                  if (isOurs(event)) event.preventDefault();
                }}
                onDrop={(event, lane) => {
                  const what = dropped(event);
                  if (!what) return;
                  event.preventDefault();
                  props.onDrop(what, track, frameAt(event, lane), pxSpan(SNAP_PX, zoom, fps));
                }}
              />
            </div>
          ))}
          <NewLane
            header={HEADER}
            empty={tracks.length === 0}
            carrying={carrying}
            onDrop={(dragged, px) =>
              props.onDrop(dragged, null, pxToFrames(px, zoom, fps), pxSpan(SNAP_PX, zoom, fps))
            }
          />
          <div
            className="pointer-events-none absolute top-0 bottom-0 z-20 w-px bg-red-500"
            style={{ left: HEADER + framesToPx(playhead, zoom, fps) }}
            aria-hidden
          />
        </div>
      </div>
    </div>
  );
}
