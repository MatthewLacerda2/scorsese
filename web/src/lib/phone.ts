// Whether the window is a phone's: narrower than Tailwind's `md`, the
// breakpoint the sidebar pages already fold at (`home/SidebarLayout.tsx`).
// Where a layout differs in more than its styles — the editor's, whose chat
// must stay one mounted panel (#1570) — it asks this rather than drawing both.
// No `matchMedia` (a server render) reads as not a phone.

import { useSyncExternalStore } from "react";

/** Tailwind's `md`, below which the window is a phone's. */
export const PHONE_QUERY = "(max-width: 47.99rem)";

const query = () => (typeof matchMedia === "function" ? matchMedia(PHONE_QUERY) : null);

function subscribe(changed: () => void): () => void {
  const list = query();
  list?.addEventListener?.("change", changed);
  return () => list?.removeEventListener?.("change", changed);
}

const narrow = () => query()?.matches ?? false;

/** Whether the window is a phone's now, following it as it turns or resizes. */
export function usePhone(): boolean {
  return useSyncExternalStore(subscribe, narrow, () => false);
}
