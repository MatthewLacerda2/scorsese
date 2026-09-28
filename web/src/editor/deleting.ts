// The Delete key: the selected clip comes off the timeline (`clip_remove`).
//
// Its asset stays in the project and nothing closes up behind it — the tool's
// own rules (docs/mcp.md). Backspace does the same, since a Mac keyboard's
// "delete" is Backspace. Never while somebody is typing: the inspector's fields
// and the chat box take both keys for themselves.

import { useEffect } from "react";
import type { Edit } from "./project";

/** Whatever has the keyboard, as far as this needs to know. */
interface Focused {
  tagName?: string;
  isContentEditable?: boolean;
}

/** Whether a key pressed here is somebody typing rather than a command. */
export function isTyping(target: Focused | null): boolean {
  if (!target) return false;
  if (target.isContentEditable) return true;
  return ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName ?? "");
}

/** Whether `key` is one that deletes. */
export function deletes(key: string): boolean {
  return key === "Delete" || key === "Backspace";
}

/** Removes `selected` when Delete or Backspace is pressed outside a text field,
 * and lets go of it once the server has. */
export function useDeleteKey(
  selected: string | null,
  run: (edit: Edit) => Promise<unknown>,
  deselect: () => void,
) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!selected || !deletes(event.key)) return;
      if (isTyping(event.target as Focused | null)) return;
      event.preventDefault();
      const edit: Edit = { tool: "clip_remove", args: { clips: [selected] }, edit: true };
      void run(edit).then((answer) => {
        if (answer) deselect();
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selected, run, deselect]);
}
