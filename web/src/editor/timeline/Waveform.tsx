// A sound clip's waveform (#1572): the slice of its file's peaks the clip
// plays, a bar every few pixels, below the name strip. The peaks are fetched
// once per file and kept, so trimming, zooming or scrolling only redraws; one
// still being read is asked for again when its job is done.

import { useQuery } from "@tanstack/react-query";
import { memo, useMemo } from "react";
import { api } from "@/api";
import { useServerEvents } from "@/app/events";
import { queryKeys } from "@/app/queryKeys";
import { useLibraryTile } from "../assets/AssetTile";
import { barsOf, type Peaks, parsePeaks, readAgain, retryPeaks, sliceOf } from "./peaks";

/** The stretch of its sound a clip plays, in seconds, and how wide it is drawn. */
export interface Span {
  from: number;
  length: number;
  width: number;
}

/** The peaks of the file with these bytes, once read: asked for again when
 * the job reading them is done, never given up on. */
function usePeaks(sha256: string | undefined): Peaks | undefined {
  const tile = useLibraryTile(sha256);
  const url = tile?.peaks;
  const query = useQuery({
    queryKey: queryKeys.peaks(url),
    queryFn: async () => parsePeaks(await api.library.peaks(url as string)),
    enabled: url !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
    retry: retryPeaks,
  });
  useServerEvents((event) => {
    if (url !== undefined && !query.data && readAgain(event, tile?.id)) query.refetch();
  });
  return query.data ?? undefined;
}

/** Bars of `levels` (0–1), each centred on the middle line, in a box one unit
 * tall: one path, however many bars, so a long clip is one node. */
const Bars = memo(function Bars({ levels }: { levels: number[] }) {
  const d = levels
    .map((level, at) => {
      const height = Math.max(level, 0.04);
      return `M${at} ${(1 - height) / 2}h0.667v${height}h-0.667z`;
    })
    .join("");
  return (
    <svg
      className="h-full w-full fill-screen/55"
      viewBox={`0 0 ${levels.length} 1`}
      preserveAspectRatio="none"
      aria-hidden
    >
      <path d={d} data-bars={levels.join(" ")} />
    </svg>
  );
});

export function Waveform(props: { sha256: string | undefined; span: Span; hue: string }) {
  const peaks = usePeaks(props.sha256);
  const { from, length, width } = props.span;
  // Cut again only when the slice or its width changes: a drag elsewhere in
  // the lane re-renders this clip with the same span.
  const levels = useMemo(
    () => peaks && sliceOf(peaks, from, length, barsOf(peaks, length, width)),
    [peaks, from, length, width],
  );
  return (
    <span className={`pointer-events-none absolute inset-0 ${props.hue}`} aria-hidden>
      {levels && (
        <span className="absolute inset-x-0 top-4 bottom-0.5">
          <Bars levels={levels} />
        </span>
      )}
    </span>
  );
}
