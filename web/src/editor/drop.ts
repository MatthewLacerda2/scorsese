// One of the project's assets dropped on the timeline becomes a clip, placed
// where it landed (`place_clip`), snapped by either edge as the desktop app's
// drop is. A library file reaches the assets first, through the Library modal
// (#702), so a drop never imports.
//
// **A drop never needs a track made first** (#765, #771). Dropped below the
// last lane, on an empty timeline, or on a lane of the other kind — a sound on
// a picture lane — it makes a lane of the kind the asset needs (`track_new`,
// the tool an assistant uses) and places the clip there. Two calls rather than
// one, so a refused placement leaves the new, empty lane where it is and shows
// the refusal, as any refused drop does.

import { useQueryClient } from "@tanstack/react-query";
import type { EditorProject, Track } from "@/api";
import type { Dragged } from "./assets/dragged";
import type { EditOutcome } from "./project";
import { editorKey } from "./project";
import { dropLength, dropStart, laneFor, placeArguments, targets } from "./timeline/drag";

/** Runs one edit and hands back the server's answer, or `null` when refused. */
type Run = EditOutcome["run"];

/** Clip ids in a project — to find the one a placement added. */
function clipIds(project: EditorProject | undefined | null): Set<string> {
  return new Set(
    (project?.document.tracks ?? []).flatMap((track) => track.clips.map((clip) => clip.id)),
  );
}

/** The first of `after`'s tracks that `before` did not have. */
function addedTrack(before: EditorProject, after: EditorProject | undefined | null) {
  const had = new Set((before.document.tracks ?? []).map((track) => track.id));
  return after?.document.tracks?.find((track) => !had.has(track.id));
}

/** Whether `track` can take what was dragged: picture on a video lane, sound
 * on an audio one. `null` — below the last lane, or an empty timeline — takes
 * nothing, so the drop makes a lane. */
export function takes(track: Track | null, kind: string): track is Track {
  return track !== null && track.kind === laneFor(kind);
}

/** What a drop of `dragged` at frame `pointed` does to `before`: places it on
 * `track` when that lane takes it, or on a new lane of its kind. Hands back the
 * id of the clip it added, or `null` when an edit was refused. */
export async function dropOnto(
  run: Run,
  before: EditorProject,
  dragged: Dragged,
  track: Track | null,
  pointed: number,
  reach: number,
  playhead: number,
): Promise<string | null> {
  const { document } = before;
  const asset = document.assets?.find((found) => found.id === dragged.asset);
  const kind = asset?.kind ?? dragged.kind;
  let onto = takes(track, kind) ? track : null;
  let current = before;
  if (!onto) {
    const made = await run({ tool: "track_new", args: { kind: laneFor(kind) }, edit: true });
    onto = addedTrack(before, made?.project) ?? null;
    if (!onto || !made?.project) return null;
    current = made.project;
  }
  const fps = document.timeline_fps;
  const length = dropLength(asset, fps);
  const start = dropStart(pointed, length, targets(document, playhead), reach);
  const args = placeArguments(asset, dragged.asset, onto.id, start, fps);
  const placed = await run({ tool: "place_clip", args, edit: true });
  const had = clipIds(current);
  return [...clipIds(placed?.project)].find((clip) => !had.has(clip)) ?? null;
}

export function useDrop(
  id: number,
  edit: EditOutcome,
  playhead: number,
  select: (clip: string) => void,
) {
  const queryClient = useQueryClient();

  return async (dragged: Dragged, track: Track | null, pointed: number, reach: number) => {
    const before = queryClient.getQueryData<EditorProject>(editorKey(id));
    if (!before) return;
    const added = await dropOnto(edit.run, before, dragged, track, pointed, reach, playhead);
    if (added) select(added);
  };
}
