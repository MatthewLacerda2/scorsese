// A sound clip's waveform (#1572): the slice of its file's peaks the clip
// plays, a bar every few pixels, below the name strip. The peaks are fetched
// once per file and kept, so trimming, zooming or scrolling only redraws.

import { useQuery } from "@tanstack/react-query";
import { ApiError, api } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useLibraryTile } from "../assets/AssetTile";
import { barsOf, type Peaks, parsePeaks, sliceOf } from "./peaks";

/** How often to ask again while the server is still reading a sound. */
const PENDING_RETRY_MS = 3000;

/** The stretch of its sound a clip plays, in seconds, and how wide it is drawn. */
export interface Span {
  from: number;
  length: number;
  width: number;
}

/** The peaks of the file with these bytes, once read. */
function usePeaks(sha256: string | undefined): Peaks | undefined {
  const url = useLibraryTile(sha256)?.peaks;
  const query = useQuery({
    queryKey: queryKeys.peaks(url),
    queryFn: async () => parsePeaks(await api.library.peaks(url as string)),
    enabled: url !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
    // `404` while the server is still reading the sound: ask again shortly.
    retry: (count, error) => error instanceof ApiError && error.status === 404 && count < 40,
    retryDelay: PENDING_RETRY_MS,
  });
  return query.data ?? undefined;
}

/** Bars of `levels` (0–1), each centred on the middle line, in a box one unit tall. */
function Bars({ levels }: { levels: number[] }) {
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
}

export function Waveform(props: { sha256: string | undefined; span: Span; hue: string }) {
  const peaks = usePeaks(props.sha256);
  const { from, length, width } = props.span;
  return (
    <span className={`pointer-events-none absolute inset-0 ${props.hue}`} aria-hidden>
      {peaks && (
        <span className="absolute inset-x-0 top-4 bottom-0.5">
          <Bars levels={sliceOf(peaks, from, length, barsOf(width))} />
        </span>
      )}
    </span>
  );
}
