// The preview played in pieces (#1448), as React sees it: a player made when
// play is pressed, dropped with everything it loaded when the revision, the
// quality or the cut's length moves, and told about its jobs from the event
// stream. `player.ts` has how the pieces are asked for and played.

import { type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { api, type Fps } from "@/api";
import { useServerEvents } from "@/app/events";
import { toSeconds } from "../../timeline/time";
import type { Quality } from "../quality";
import { secondsAhead, short } from "./plan";
import { PiecePlayer } from "./player";

/** How long the player may sit paused before what it is making is cancelled. */
const REST_MS = 30_000;

/** What the picture shows of the pieces. */
export interface PiecesView {
  /** Play was pressed for this revision, so the video is the picture once it covers the playhead. */
  started: boolean;
  /** The playhead's frame is loaded: the video can show it. */
  covered: boolean;
  /** Less than ten seconds loaded ahead: "Generating preview". */
  short: boolean;
  failed: boolean;
}

const NONE: PiecesView = { started: false, covered: false, short: false, failed: false };

/** The buffered stretches of `element`, as `[start, end]` seconds. */
function stretches(element: HTMLVideoElement): [number, number][] {
  const { buffered } = element;
  return Array.from({ length: buffered.length }, (_, at) => [buffered.start(at), buffered.end(at)]);
}

export function usePieces(
  video: RefObject<HTMLVideoElement | null>,
  id: number,
  revision: number,
  deliver: string,
  quality: Quality,
  total: number,
  fps: Fps,
) {
  const player = useRef<PiecePlayer | null>(null);
  const rest = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [view, setView] = useState<PiecesView>(NONE);

  const look = useCallback(() => {
    const element = video.current;
    const playing = player.current;
    if (!element || !playing) return setView(NONE);
    const loaded = stretches(element);
    const at = element.currentTime;
    setView({
      started: true,
      covered: secondsAhead(loaded, at) > 0,
      short: short(loaded, at, toSeconds(total, fps)),
      failed: playing.failed,
    });
  }, [video, total, fps]);

  // Whatever was loaded is of another revision, quality or length now.
  // biome-ignore lint/correctness/useExhaustiveDependencies: these are what make the loaded pieces stale
  useEffect(() => {
    setView(NONE);
    return () => {
      clearTimeout(rest.current);
      player.current?.dispose();
      player.current = null;
    };
  }, [id, revision, deliver, quality, total, fps.num, fps.den]);

  useServerEvents((event) => {
    if (event.type === "job") player.current?.nudge(event.id, event.state);
  });

  /** Play from `frame`: the first press makes the player. */
  const play = (frame: number) => {
    const element = video.current;
    if (!element) return;
    clearTimeout(rest.current);
    if (!player.current) {
      const server = {
        ask: (stream: "picture" | "sound", range?: string) =>
          api.renders.preview(id, { resolution: deliver, quality, stream, range }),
        bytes: api.renders.bytes,
        cancel: (job: number) =>
          void api.jobs.cancel(job).catch((error) => console.error("preview: cancel", error)),
      };
      player.current = new PiecePlayer(element, server, total, fps, look);
    }
    element.currentTime = toSeconds(frame, fps);
    player.current.play(frame);
    look();
    // Waits for the first piece by itself; a refusal (no gesture) leaves it paused.
    element.play().catch(() => {});
  };

  /** The playhead moved while paused. */
  const seek = (frame: number) => {
    player.current?.seek(frame);
    look();
  };

  /** Playing: the element's clock moved. */
  const tick = () => {
    player.current?.tick();
    look();
  };

  /** Paused: after a while, stop making pieces nobody may watch. */
  const paused = () => {
    clearTimeout(rest.current);
    rest.current = setTimeout(() => player.current?.rest(), REST_MS);
  };

  return { view, play, seek, tick, paused, look };
}
