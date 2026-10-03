// Removing an asset or a lane (#396): a plain confirm that lists the clips
// going with it, then one `asset_remove` or `track_remove` naming exactly
// those. The list is the confirmation — the server refuses any other one — so
// a timeline that moved on since it was drawn is refused, not guessed at.

import type { ProjectDocument, Track } from "@/api";
import type { Edit } from "./project";

/** What to ask before a removal, and the edit a yes sends. */
export interface Removal {
  question: string;
  edit: Edit;
}

/** Every lane in the document: the timeline's, then each group's. */
function everyTrack(document: ProjectDocument): Track[] {
  const groups = (document.assets ?? []).flatMap((asset) => asset.group?.tracks ?? []);
  return [...(document.tracks ?? []), ...groups];
}

/** The ids of the clips that show `asset`, groups' members included. */
export function showing(document: ProjectDocument, asset: string): string[] {
  return everyTrack(document).flatMap((track) =>
    track.clips.filter((clip) => clip.asset === asset).map((clip) => clip.id),
  );
}

/** `"a", "b"` for a confirm, at most `shown` of them and a count of the rest. */
function listed(ids: string[], shown = 8): string {
  const named = ids.slice(0, shown).map((id) => `“${id}”`);
  const more = ids.length - named.length;
  return more > 0 ? `${named.join(", ")} and ${more} more` : named.join(", ");
}

/** Removing `asset`, and every clip that shows it. */
export function assetRemoval(document: ProjectDocument, asset: string): Removal {
  const clips = showing(document, asset);
  const question =
    clips.length === 0
      ? `Remove “${asset}” from the project?`
      : `Remove “${asset}” from the project? ${clips.length === 1 ? "This clip uses it and will be deleted too" : `These ${clips.length} clips use it and will be deleted too`}: ${listed(clips)}.`;
  return { question, edit: { tool: "asset_remove", args: { asset, clips }, edit: true } };
}

/** Removing `track`, and every clip on it. */
export function trackRemoval(track: Track): Removal {
  const clips = track.clips.map((clip) => clip.id);
  const name = track.name ?? track.id;
  const question =
    clips.length === 0
      ? `Remove the track “${name}”?`
      : `Remove the track “${name}”? ${clips.length === 1 ? "The clip on it will be deleted too" : `The ${clips.length} clips on it will be deleted too`}: ${listed(clips)}.`;
  return {
    question,
    edit: { tool: "track_remove", args: { track: track.id, clips }, edit: true },
  };
}

/** Asks, and sends the edit only on a yes. */
export function confirmThen(
  removal: Removal,
  run: (edit: Edit) => Promise<unknown>,
  ask: (question: string) => boolean = (question) => window.confirm(question),
) {
  if (ask(removal.question)) void run(removal.edit);
}
