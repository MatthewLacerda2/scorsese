// The strip of seconds above the lanes. Pressing on it puts the playhead
// there, and dragging along it scrubs.

import type { PointerEvent } from "react";
import type { Fps } from "@/api";
import { framesToPx, rate, type Zoom } from "./time";

interface Props {
  width: number;
  zoom: Zoom;
  fps: Fps;
  onScrub: (event: PointerEvent, ruler: Element) => void;
}

export function Ruler({ width, zoom, fps, onScrub }: Props) {
  // A label every second when there is room for one, every five or ten when not.
  const every = zoom.pxPerSecond >= 40 ? 1 : zoom.pxPerSecond >= 20 ? 5 : 10;
  const seconds = Math.ceil(width / zoom.pxPerSecond);
  const marks = [];
  for (let second = 0; second <= seconds; second += every) marks.push(second);
  return (
    <div
      className="relative h-6 shrink-0 cursor-ew-resize select-none border-b text-[10px] text-muted-foreground"
      style={{ width }}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        onScrub(event, event.currentTarget);
      }}
      onPointerMove={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          onScrub(event, event.currentTarget);
        }
      }}
    >
      {marks.map((second) => (
        <span
          key={second}
          className="absolute top-0 h-full border-l pl-1"
          style={{ left: framesToPx(second * rate(fps), zoom, fps) }}
        >
          {second >= 60
            ? `${Math.floor(second / 60)}:${String(second % 60).padStart(2, "0")}`
            : `${second}s`}
        </span>
      ))}
    </div>
  );
}
