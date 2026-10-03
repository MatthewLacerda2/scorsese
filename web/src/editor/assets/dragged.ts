// What is being dragged from the assets panel onto a lane: one of the
// project's assets — a library file comes into the project first, through the
// Library modal (#702), so nothing else is ever dragged. Carried in the
// browser's own drag-and-drop under a type of our own, so a file dragged in
// from the desktop is never mistaken for one.

import type { DragEvent } from "react";

export const DRAG_TYPE = "application/x-scorsese-asset";

export type Dragged = { from: "project"; asset: string; kind: string };

export function carry(event: DragEvent, dragged: Dragged) {
  event.dataTransfer.setData(DRAG_TYPE, JSON.stringify(dragged));
  event.dataTransfer.effectAllowed = "copy";
}

/** Whether a drag is ours — readable during `dragover`, when the data is not. */
export function isOurs(event: DragEvent): boolean {
  return event.dataTransfer.types.includes(DRAG_TYPE);
}

/** What was dropped, or `null` for anything that is not ours. */
export function dropped(event: DragEvent): Dragged | null {
  const text = event.dataTransfer.getData(DRAG_TYPE);
  if (!text) return null;
  try {
    return JSON.parse(text) as Dragged;
  } catch {
    return null;
  }
}
