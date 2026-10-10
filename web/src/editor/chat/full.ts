// Whether the assistant has the whole project page (#1027): the chat alone, in
// one readable column, the way Claude's own chat looks — or the editor, with
// the chat as its sidebar. A user who prompts the video into existence lives in
// the first; one who scrubs and nudges, in the second.
//
// Remembered per project in this browser, the way the effort level is
// (`effort.ts`): `localStorage` can throw, so every access is caught, and a
// failure — or nothing stored — reads as the editor, which is how a project
// opens until the user turns full mode on.

import { useState } from "react";
import { browserStorage } from "@/i18n/language";

/** The `localStorage` key for `project`'s mode. */
export function fullKey(project: number): string {
  return `scorsese-chat-full-${project}`;
}

/** Whether `project` was last left in full mode. */
export function storedFull(
  storage: Pick<Storage, "getItem"> | undefined,
  project: number,
): boolean {
  try {
    return storage?.getItem(fullKey(project)) === "true";
  } catch {
    return false;
  }
}

/** Remember `project`'s mode. A storage that refuses only means it is not remembered. */
export function saveFull(
  storage: Pick<Storage, "setItem"> | undefined,
  project: number,
  full: boolean,
): void {
  try {
    storage?.setItem(fullKey(project), String(full));
  } catch {
    // Private window or storage disabled: the mode lasts this page only.
  }
}

/** `project`'s mode as React state, remembered in this browser as it changes. */
export function useFull(project: number): [boolean, (full: boolean) => void] {
  const [full, setFull] = useState(() => storedFull(browserStorage(), project));
  return [
    full,
    (next) => {
      setFull(next);
      saveFull(browserStorage(), project, next);
    },
  ];
}
