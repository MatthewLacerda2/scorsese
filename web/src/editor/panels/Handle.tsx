// A panel's inner edge, to drag. It is a strip of its own in the grid, so it
// takes no presses from what is beside it — the timeline's handle sits above
// the Ruler, which scrubs, rather than on it — and it shows `col-resize` or
// `row-resize`, never the `ew-resize` the Ruler and a clip's trim edges show.
// Double-click puts the panel back to its default; the arrow keys nudge it.

import type { KeyboardEvent, PointerEvent } from "react";
import { cn } from "@/lib/utils";
import { remPx } from "./usePanels";

/** Which way the panel grows as its edge is dragged. */
export type Grows = "right" | "left" | "up";

interface Props {
  grows: Grows;
  /** The panel's size now, in rem. */
  size: number;
  label: string;
  className?: string;
  onSize: (size: number) => void;
  onReset: () => void;
}

/** A keypress nudges by this much, in rem. */
const NUDGE = 1;

export function Handle({ grows, size, label, className, onSize, onReset }: Props) {
  const across = grows !== "up";
  const sign = grows === "right" ? 1 : -1;
  const at = (event: PointerEvent) => (across ? event.clientX : event.clientY);

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    const element = event.currentTarget;
    element.setPointerCapture(event.pointerId);
    const start = at(event);
    const rem = remPx();
    const move = (moved: globalThis.PointerEvent) =>
      onSize(size + (sign * ((across ? moved.clientX : moved.clientY) - start)) / rem);
    const stop = () => {
      element.removeEventListener("pointermove", move);
      element.removeEventListener("pointerup", stop);
      element.removeEventListener("pointercancel", stop);
    };
    element.addEventListener("pointermove", move);
    element.addEventListener("pointerup", stop);
    element.addEventListener("pointercancel", stop);
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const keys: Record<string, number> = across
      ? { ArrowRight: 1, ArrowLeft: -1 }
      : { ArrowDown: 1, ArrowUp: -1 };
    const step = keys[event.key];
    if (step === undefined) return;
    event.preventDefault();
    // A key moves the edge the way it points, as a drag would.
    onSize(size + sign * step * NUDGE);
  };

  return (
    // biome-ignore lint/a11y/useSemanticElements: an <hr> cannot be focused or dragged; a focusable separator is the ARIA pattern for a splitter.
    <div
      role="separator"
      aria-orientation={across ? "vertical" : "horizontal"}
      aria-label={label}
      aria-valuenow={Math.round(size * 10) / 10}
      tabIndex={0}
      className={cn(
        "group flex touch-none select-none items-center justify-center outline-none",
        across ? "cursor-col-resize" : "cursor-row-resize",
        className,
      )}
      onPointerDown={onPointerDown}
      onDoubleClick={onReset}
      onKeyDown={onKeyDown}
    >
      <div
        className={cn(
          "bg-border transition-colors group-hover:bg-primary/60 group-focus-visible:bg-primary group-active:bg-primary",
          across ? "h-full w-px" : "h-px w-full",
        )}
      />
    </div>
  );
}
