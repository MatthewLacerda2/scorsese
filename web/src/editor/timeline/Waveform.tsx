// A sound clip's waveform (#1572): the slice of its file's peaks the clip
// plays, a bar every few pixels, below the name strip. The peaks are fetched
// once per file and kept, so trimming, zooming or scrolling only redraws.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { memo, useMemo } from "react";
import { api } from "@/api";
import { useServerEvents } from "@/app/events";
import { queryKeys } from "@/app/queryKeys";
import { useLibraryTile } from "../assets/AssetTile";
import { barsOf, type Peaks, parsePeaks, readsPeaks, sliceOf } from "./peaks";

/** The stretch of its sound a clip plays, in seconds, and how wide it is drawn. */
export interface Span {
  from: number;
  length: number;
  width: number;
}

/**
 * The peaks of the file with these bytes, once read, and the tile they belong
 * to while they are not.
 */
function usePeaks(sha256: string | undefined): {
  peaks?: Peaks;
  waiting?: { item: number; url: string };
} {
  const tile = useLibraryTile(sha256);
  const url = tile?.peaks;
  const query = useQuery({
    queryKey: queryKeys.peaks(url),
    queryFn: async () => parsePeaks(await api.library.peaks(url as string)),
    enabled: url !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
    // A `404` while the server is still reading them is asked again by `Awaiting`.
    retry: false,
  });
  const peaks = query.data ?? undefined;
  if (peaks || !tile || url === undefined || query.isFetching) return { peaks };
  return { waiting: { item: tile.id, url } };
}

/**
 * Asks for a sound's peaks again once the job reading them is done, as the
 * live stream says, however long that job waited in the queue. Mounted only
 * while they are missing, so a drawn waveform holds no listener.
 */
function Awaiting({ item, url }: { item: number; url: string }) {
  const client = useQueryClient();
  useServerEvents((event) => {
    if (readsPeaks(event, item)) void client.invalidateQueries({ queryKey: queryKeys.peaks(url) });
  });
  return null;
}

/** Bars of `levels` (0–1), each centred on the middle line, in a box one unit tall. */
const Bars = memo(function DrawnBars({ levels }: { levels: number[] }) {
  return (
    <svg
      className="h-full w-full fill-screen/55"
      viewBox={`0 0 ${levels.length} 1`}
      preserveAspectRatio="none"
      aria-hidden
    >
      {levels.map((level, at) => {
        const height = Math.max(level, 0.04);
        return (
          <rect
            // biome-ignore lint/suspicious/noArrayIndexKey: a bar is its position
            key={at}
            x={at}
            y={(1 - height) / 2}
            width={2 / 3}
            height={height}
            data-bar={level}
          />
        );
      })}
    </svg>
  );
});

export function Waveform(props: { sha256: string | undefined; span: Span; hue: string }) {
  const { peaks, waiting } = usePeaks(props.sha256);
  const { from, length, width } = props.span;
  // Cut again only when the slice or its width changes, not on every
  // pointer move of a drag elsewhere in the lane.
  const levels = useMemo(
    () => peaks && sliceOf(peaks, from, length, barsOf(peaks, length, width)),
    [peaks, from, length, width],
  );
  return (
    <span className={`pointer-events-none absolute inset-0 ${props.hue}`} aria-hidden>
      {waiting && <Awaiting {...waiting} />}
      {levels && (
        <span className="absolute inset-x-0 top-4 bottom-0.5">
          <Bars levels={levels} />
        </span>
      )}
    </span>
  );
}
