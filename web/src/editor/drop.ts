// One of the project's assets dropped on a lane becomes a clip, placed where
// it landed (`place_clip`), snapped by either edge as the desktop app's drop
// is. A library file reaches the assets first, through the Library modal
// (#702), so a drop never imports.

import { useQueryClient } from "@tanstack/react-query";
import type { EditorProject, Track } from "@/api";
import type { Dragged } from "./assets/dragged";
import type { EditOutcome } from "./project";
import { editorKey } from "./project";
import { dropLength, dropStart, placeArguments, targets } from "./timeline/drag";

/** Clip ids in a project — to find the one a placement added. */
function clipIds(project: EditorProject | undefined): Set<string> {
  return new Set(
    (project?.document.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.id)),
  );
}

export function useDrop(
  id: number,
  edit: EditOutcome,
  playhead: number,
  select: (clip: string) => void,
) {
  const queryClient = useQueryClient();
  const current = () => queryClient.getQueryData<EditorProject>(editorKey(id));

  return async (dragged: Dragged, track: Track, pointed: number, reach: number) => {
    const assetId = dragged.asset;
    const before = current();
    if (!before) return;
    const { document } = before;
    const asset = document.assets?.find((found) => found.id === assetId);
    const length = dropLength(asset, document.timeline_fps);
    const start = dropStart(pointed, length, targets(document, playhead), reach);
    const args = placeArguments(asset, assetId, track.id, start, document.timeline_fps);
    const placed = await edit.run({ tool: "place_clip", args, edit: true });
    const had = clipIds(before);
    const added = [...clipIds(placed?.project ?? undefined)].find((clip) => !had.has(clip));
    if (added) select(added);
  };
}
