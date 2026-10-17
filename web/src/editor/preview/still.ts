// The preview's still picture: the frame under the playhead, composited by the
// server's `still` tool — the same compositor a render uses, so what is shown
// is what the render delivers. Asked for once the playhead rests (a scrub is
// dozens of positions, and each is a round trip), and kept on screen while
// the next one comes so the picture never blinks out.

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { ApiError, api } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useT } from "@/i18n/I18nProvider";
import { useSettled } from "@/lib/useSettled";
import { clamp } from "@/lib/utils";

/** How long the playhead has to rest before its frame is asked for. */
const SETTLE_MS = 200;

/**
 * The picture, or why there is none: what the tool said when it drew nothing
 * (an empty timeline, say), or the editor's own words when it refused — its
 * refusal is a sentence for the assistant, never put under the picture (#1439).
 */
export type Still = { png: string } | { none: string };

/**
 * The frame to draw for a playhead at `frame` on a timeline `length` frames
 * long: a playhead resting at the end (or past it) shows the last frame, as
 * every editor does, and the key then matches the frame really drawn (#1276).
 */
export function clampFrame(frame: number, length: number): number {
  return clamp(frame, 0, length - 1);
}

export function useStill(
  id: number,
  revision: number,
  frame: number,
  length: number,
  raster: string,
  enabled: boolean,
) {
  const at = clampFrame(useSettled(frame, SETTLE_MS), length);
  const failed = useT().editor.preview.failed;
  return useQuery({
    queryKey: queryKeys.still(id, revision, at, raster),
    enabled,
    placeholderData: keepPreviousData,
    // A frame of a revision never changes.
    staleTime: Infinity,
    queryFn: async (): Promise<Still> => {
      try {
        // A bare whole number is a frame on the timeline, to the `still` tool.
        const answer = await api.editor.tool(id, "still", { at: String(at), resolution: raster });
        const image = answer.said.find((part) => part.image)?.image;
        return image
          ? { png: `data:image/png;base64,${image}` }
          : { none: answer.said[0]?.text ?? "" };
      } catch (error) {
        if (error instanceof ApiError && error.status === 422) {
          console.error("preview: the still was refused", error.message);
          return { none: failed };
        }
        throw error;
      }
    },
  });
}
