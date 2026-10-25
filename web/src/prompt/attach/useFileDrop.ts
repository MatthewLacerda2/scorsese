// Files dropped from outside the page onto an element, and whether some are
// being dragged over it now (for a "drop here" look). Only a drag carrying
// `Files` is taken: the editor's own drags — an asset onto the timeline — go
// by untouched, neither prevented nor stopped. A drag an inner drop zone has
// handled (it prevented the default), or over a portal — a dialog such as the
// editor's Library modal, whose React events still bubble here — is left to
// that zone: it is not uploaded twice, and no overlay shows behind it.

import { type DragEvent, useRef, useState } from "react";
import { carriesFiles } from "@/lib/upload/formats";

export function useFileDrop(onFiles: (files: File[]) => void, onEnter?: () => void) {
  const [dragging, setDragging] = useState(false);
  // Entering a child leaves the parent first: count, rather than flicker.
  const depth = useRef(0);
  const ours = (event: DragEvent) =>
    carriesFiles(Array.from(event.dataTransfer.types)) &&
    !event.defaultPrevented &&
    event.target instanceof Node &&
    event.currentTarget.contains(event.target);
  const handlers = {
    onDragEnter: (event: DragEvent) => {
      if (!ours(event)) return;
      depth.current += 1;
      setDragging(true);
      onEnter?.();
    },
    onDragLeave: (event: DragEvent) => {
      if (!ours(event)) return;
      depth.current = Math.max(0, depth.current - 1);
      if (depth.current === 0) setDragging(false);
    },
    onDragOver: (event: DragEvent) => {
      if (!ours(event)) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
    },
    onDrop: (event: DragEvent) => {
      if (!ours(event)) return;
      event.preventDefault();
      depth.current = 0;
      setDragging(false);
      onFiles(Array.from(event.dataTransfer.files));
    },
  };
  return { dragging, handlers };
}
