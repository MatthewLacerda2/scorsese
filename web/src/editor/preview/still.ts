// The preview's still picture: the frame under the playhead, composited by the
// server's `still` tool — the same compositor a render uses, so what is shown
// is what the render delivers. Asked for once the playhead rests (a scrub is
// dozens of positions, and each is a round trip), and kept on screen while
// the next one comes so the picture never blinks out.

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { ApiError, api } from "@/api";

/** How long the playhead has to rest before its frame is asked for. */
const SETTLE_MS = 200;

/** `value`, once it has stopped changing for `ms`. */
export function useSettled<T>(value: T, ms: number): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return settled;
}

/** The picture, or the tool's reason there is none (an empty timeline, say). */
export type Still = { png: string } | { none: string };

export function useStill(
  id: number,
  revision: number,
  frame: number,
  raster: string,
  enabled: boolean,
) {
  const at = useSettled(frame, SETTLE_MS);
  return useQuery({
    queryKey: ["projects", "still", id, revision, at, raster],
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
        if (error instanceof ApiError && error.status === 422) return { none: error.message };
        throw error;
      }
    },
  });
}
