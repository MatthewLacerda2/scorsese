// The space below the last lane: where a drop makes a new track (#765), and on
// an empty timeline the whole of it. While an asset is being carried from the
// assets panel it says so, and lights up when the pointer is over it.

import { type DragEvent, useEffect, useState } from "react";
import { useT } from "@/i18n/I18nProvider";
import { DRAG_TYPE, type Dragged, dropped, isOurs } from "../assets/dragged";

/** Whether one of the project's assets is being dragged anywhere on the page —
 * so the timeline can show where a new lane would go before the pointer
 * reaches it. */
export function useCarrying(): boolean {
  const [carrying, setCarrying] = useState(false);
  useEffect(() => {
    const start = (event: globalThis.DragEvent) =>
      setCarrying(event.dataTransfer?.types.includes(DRAG_TYPE) ?? false);
    const stop = () => setCarrying(false);
    window.addEventListener("dragstart", start);
    window.addEventListener("dragend", stop);
    window.addEventListener("drop", stop);
    return () => {
      window.removeEventListener("dragstart", start);
      window.removeEventListener("dragend", stop);
      window.removeEventListener("drop", stop);
    };
  }, []);
  return carrying;
}

interface Props {
  /** How wide the lane headers are: the drop's frame is measured past them. */
  header: number;
  /** No lanes yet, so this is the whole timeline. */
  empty: boolean;
  carrying: boolean;
  /** Something dropped `px` pixels past the head of the timeline. */
  onDrop: (dragged: Dragged, px: number) => void;
}

export function NewLane({ header, empty, carrying, onDrop }: Props) {
  const t = useT().editor.timeline;
  const [over, setOver] = useState(false);
  const text = carrying ? t.newTrack : empty ? t.empty : null;
  const drop = (event: DragEvent<HTMLDivElement>) => {
    setOver(false);
    const what = dropped(event);
    if (!what) return;
    event.preventDefault();
    onDrop(what, event.clientX - event.currentTarget.getBoundingClientRect().left - header);
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a drop target for the assets panel's drag, whose keyboard equivalent is the assistant
    <div
      className={`min-h-12 flex-1 ${carrying ? "border-2 border-dashed" : ""} ${over ? "border-primary bg-primary/5" : "border-muted-foreground/30"}`}
      data-new-lane
      onDragOver={(event) => {
        if (!isOurs(event)) return;
        event.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={drop}
    >
      {text && (
        <p className="sticky left-0 inline-block p-4 text-sm text-muted-foreground">{text}</p>
      )}
    </div>
  );
}
