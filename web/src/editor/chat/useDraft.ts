// The first message a new project was opened with but could not send
// (`prompt/draft.ts`, #1535): read once as the editor opens, its files put
// back in the tray, then forgotten, so leaving and coming back finds an
// empty box.

import { useEffect, useRef, useState } from "react";
import { type Draft, draftFor, forgetDraft } from "@/prompt/draft";

/** Project `id`'s draft, if one was left for it; its files go to `attach`. */
export function useDraft(id: number, attach: (files: File[]) => void): Draft | undefined {
  const [draft] = useState(() => draftFor(id));
  const attached = useRef(false);
  useEffect(() => {
    forgetDraft(id);
    if (!draft || attached.current) return;
    attached.current = true;
    if (draft.files.length > 0) attach(draft.files);
  }, [id, draft, attach]);
  return draft;
}
