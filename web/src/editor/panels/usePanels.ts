// The editor grid's panel sizes as React state: what the person last dragged
// each panel to (kept in this browser), the editor's measured area, and the
// sizes drawn — the first fitted to the second (`sizes.ts`).

import { type CSSProperties, useEffect, useRef, useState } from "react";
import {
  DEFAULTS,
  fit,
  HANDLE,
  type Panel,
  type Room,
  resize,
  savedSizes,
  saveSizes,
} from "./sizes";

/** Pixels in a rem on this page; 16 without a document (a test's server render). */
export function remPx(): number {
  if (typeof document === "undefined") return 16;
  return Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
}

/** `localStorage`, or `undefined` where even reading the property throws. */
function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

export function usePanels() {
  const grid = useRef<HTMLDivElement>(null);
  const [wanted, setWanted] = useState(() => savedSizes(browserStorage()));
  const [room, setRoom] = useState<Room | null>(null);

  useEffect(() => {
    const element = grid.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const rem = remPx();
      setRoom({ width: entry.contentRect.width / rem, height: entry.contentRect.height / rem });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => saveSizes(browserStorage(), wanted), [wanted]);

  const sizes = fit(wanted, room);
  // The handles have tracks of their own, between the panels and over none.
  const style: CSSProperties = {
    gridTemplateColumns: `${sizes.chat}rem ${HANDLE}rem minmax(0,1fr) ${HANDLE}rem ${sizes.assets}rem`,
    gridTemplateRows: `minmax(0,1fr) ${HANDLE}rem ${sizes.timeline}rem`,
  };
  return {
    grid,
    style,
    sizes,
    /** `panel` dragged to `size` rem, held where it may go. */
    resize: (panel: Panel, size: number) => setWanted(resize(sizes, panel, size, room)),
    /** `panel` back to today's size, as far as the room allows. */
    reset: (panel: Panel) => setWanted(resize(sizes, panel, DEFAULTS[panel], room)),
  };
}
