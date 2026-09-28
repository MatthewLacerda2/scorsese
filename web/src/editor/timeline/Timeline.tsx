// The timeline: a ruler to scrub on, one lane per track, and the clips on
// them — dragged to move, dragged by an edge to trim, and dropped onto from the
// assets panel. The desktop app's `app/src/timeline/`, in the browser.
//
// A drag is drawn where the pointer proposes, snapped (drag.ts), and becomes
// one `trim_clip` when the pointer lets go — never before, since every edit is
// a round trip to the server's tools. A refused one springs back: the page
// only ever draws the document the server holds.

import { MinusIcon, PlusIcon } from "lucide-react";
import { type DragEvent, type PointerEvent, useState } from "react";
import type { Fps, ProjectDocument, Track } from "@/api";
import { Button } from "@/components/ui/button";
import { type Dragged, dropped, isOurs } from "../assets/dragged";
import { SNAP_PX } from "./drag";
import { Lane } from "./Lane";
import { Ruler } from "./Ruler";
import { framesToPx, pxSpan, pxToFrames, toFrames, ZOOMS, type Zoom } from "./time";

/** How wide the lane headers are, in pixels. */
const HEADER = 112;

export interface TimelineProps {
  document: ProjectDocument;
  playhead: number;
  onSeek: (frame: number) => void;
  selected: string | null;
  onSelect: (clip: string | null) => void;
  /** A drag let go: `trim_clip`'s arguments. */
  onTrim: (args: Record<string, unknown>) => Promise<unknown>;
  /** Something dropped on `track` at frame `pointed`, before snapping. */
  onDrop: (dragged: Dragged, track: Track, pointed: number, reach: number) => void;
  onAddTrack: (kind: "video" | "audio") => void;
}

export function Timeline(props: TimelineProps) {
  const { document, playhead, onSeek } = props;
  const fps: Fps = document.timeline_fps;
  const [zoomAt, setZoomAt] = useState(2);
  const zoom: Zoom = { pxPerSecond: ZOOMS[zoomAt] ?? 40 };
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
      <div className="flex items-center gap-1 border-b px-2 py-1">
        <Button size="sm" variant="ghost" onClick={() => props.onAddTrack("video")}>
          <PlusIcon /> Video track
        </Button>
        <Button size="sm" variant="ghost" onClick={() => props.onAddTrack("audio")}>
          <PlusIcon /> Audio track
        </Button>
        <span className="ml-auto text-xs text-muted-foreground">Zoom</span>
        <Button
          size="icon"
          variant="ghost"
          aria-label="Zoom out"
          disabled={zoomAt === 0}
          onClick={() => setZoomAt((at) => Math.max(0, at - 1))}
        >
          <MinusIcon />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          aria-label="Zoom in"
          disabled={zoomAt === ZOOMS.length - 1}
          onClick={() => setZoomAt((at) => Math.min(ZOOMS.length - 1, at + 1))}
        >
          <PlusIcon />
        </Button>
      </div>
      <div className="relative min-h-0 flex-1 overflow-auto">
        <div className="relative" style={{ width: width + HEADER }}>
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
          {tracks.length === 0 && (
            <p className="p-4 text-sm text-muted-foreground">
              No tracks yet. Add a video track, then drag something onto it from the left.
            </p>
          )}
          {tracks.map((track) => (
            <div key={track.id} className="flex border-b">
              <div
                className="sticky left-0 z-10 flex shrink-0 flex-col justify-center border-r bg-background px-2"
                style={{ width: HEADER }}
              >
                <span className="truncate text-xs font-medium">{track.name ?? track.id}</span>
                <span className="text-[10px] text-muted-foreground">{track.kind}</span>
              </div>
              <Lane
                track={track}
                document={document}
                width={width}
                zoom={zoom}
                playhead={playhead}
                selected={props.selected}
                onSelect={props.onSelect}
                onTrim={props.onTrim}
                onEmptyClick={(event, lane) => {
                  props.onSelect(null);
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
