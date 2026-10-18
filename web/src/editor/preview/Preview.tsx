// The preview: the frame under the playhead while editing, and the cut
// playing when asked to — with the transport under it (`Transport.tsx`).
//
// **Two pictures, one rule: the server draws both.** Until there is video of
// this revision under the playhead, the frame there is a still from the
// `still` tool; then it is the video, where scrubbing is seeking. Both are
// drawn at the chosen preview quality (`quality.ts`). Real-time compositing
// in the browser is not attempted.
//
// **The video is made only when play is pressed** (#1448): a cut of thirty
// seconds or less whole (`playable.ts`), a longer one in pieces from the
// playhead, about ten seconds ahead (`pieces/`). Pressed while an edit is in
// flight — the assistant at work, or a hand-edit on its way — it plays
// nothing and says why: a preview of a revision that is moving would be
// dropped the moment it landed.

import { MaximizeIcon, MinimizeIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ProjectDocument } from "@/api";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { clamp } from "@/lib/utils";
import { JobProgressBar } from "../JobProgressBar";
import { toFrames, toSeconds } from "../timeline/time";
import { playedWhole } from "./pieces/plan";
import { canPlayPieces } from "./pieces/player";
import { usePieces } from "./pieces/usePieces";
import { usePlayable, waiting } from "./playable";
import { previewRaster, type Quality, savedQuality, saveQuality } from "./quality";
import { useStill } from "./still";
import { Transport } from "./Transport";

interface Props {
  projectId: number;
  revision: number;
  document: ProjectDocument;
  playhead: number;
  onSeek: (frame: number) => void;
  /** The size the film is delivered at, `WIDTHxHEIGHT`: what the preview is a fraction of. */
  deliver: string;
  /** An edit is in flight — the assistant's or a hand-edit — so play says it can't yet. */
  changing?: boolean;
}

/** Where the cut ends: the last picture, since picture decides a render's length. */
function cutLength(document: ProjectDocument): number {
  const tracks = document.tracks ?? [];
  const ends = (kind?: string) =>
    tracks
      .filter((track) => !kind || track.kind === kind)
      .flatMap((track) => track.clips.map((clip) => clip.start + clip.duration));
  return Math.max(0, ...(ends("video").length ? ends("video") : ends()));
}

export function Preview(props: Props) {
  const { projectId, revision, document, playhead, onSeek, deliver, changing = false } = props;
  const t = useT().editor.preview;
  const fps = document.timeline_fps;
  const total = cutLength(document);
  const [quality, setQuality] = useState<Quality>(savedQuality);
  const raster = previewRaster(deliver, quality);
  const video = useRef<HTMLVideoElement>(null);
  const whole = playedWhole(total, fps) || !canPlayPieces();
  const { playable, ask } = usePlayable(projectId, revision, deliver, quality);
  const pieces = usePieces(video, projectId, revision, deliver, quality, total, fps);
  const showing = whole ? playable.state === "ready" : pieces.view.covered;
  const still = useStill(projectId, revision, playhead, total, raster, !showing && total > 0);
  const stage = useRef<HTMLDivElement>(null);
  const [maximized, setMaximized] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [wanted, setWanted] = useState(false);
  const [refused, setRefused] = useState(false);
  const [width, height] = raster.split("x").map(Number);
  const ready = whole && playable.state === "ready";

  // The edit landed: play may be pressed again.
  useEffect(() => {
    if (!changing) setRefused(false);
  }, [changing]);
  // Play was pressed before the whole cut was there: start it once it is.
  useEffect(() => {
    const element = video.current;
    if (!ready || !wanted || !element) return;
    setWanted(false);
    element.currentTime = toSeconds(playhead, fps);
    void element.play();
  }, [ready, wanted, playhead, fps]);
  // Scrubbing loaded video is seeking it; in pieces, it moves what is loaded.
  // biome-ignore lint/correctness/useExhaustiveDependencies: only a moved playhead seeks
  useEffect(() => {
    const element = video.current;
    if (!element || playing) return;
    const at = toSeconds(playhead, fps);
    if (Math.abs(element.currentTime - at) > 0.5 / (fps.num / fps.den)) element.currentTime = at;
    if (!whole) pieces.seek(playhead);
  }, [playhead, playing, fps]);

  // Full screen takes the picture and its transport together, so a cut can be
  // played there; Esc leaves it without going through the button.
  useEffect(() => {
    const sync = () => setMaximized(globalThis.document.fullscreenElement === stage.current);
    globalThis.document.addEventListener("fullscreenchange", sync);
    return () => globalThis.document.removeEventListener("fullscreenchange", sync);
  }, []);
  const maximize = () => {
    if (globalThis.document.fullscreenElement) return void globalThis.document.exitFullscreen();
    void stage.current?.requestFullscreen();
  };

  const seek = (frame: number) => onSeek(clamp(frame, 0, total));
  const toggle = () => {
    if (playing) return video.current?.pause();
    if (changing) return setRefused(true);
    if (!whole) return pieces.play(playhead);
    if (ready) return void video.current?.play();
    setWanted(true);
    void ask();
  };
  const failed = whole ? playable.state === "failed" : pieces.view.failed;
  const said = failed
    ? t.failed
    : whole && playable.state === "preparing"
      ? waiting(playable.job, t)
      : !whole && pieces.view.started && pieces.view.short
        ? t.generating
        : still.data && "none" in still.data
          ? still.data.none
          : total === 0
            ? t.empty
            : null;

  return (
    <div ref={stage} className="flex h-full min-h-0 flex-col gap-2 bg-background p-3">
      <div className="group relative flex min-h-0 flex-1 items-center justify-center">
        <div
          className="relative max-h-full max-w-full overflow-hidden rounded-md bg-screen"
          style={{ aspectRatio: `${width} / ${height}`, height: "100%" }}
        >
          {/* biome-ignore lint/a11y/useMediaCaption: a preview of the user's own cut has no captions to offer */}
          <video
            ref={video}
            src={ready ? playable.render.file : undefined}
            className={showing ? "size-full object-contain" : "hidden"}
            preload="auto"
            onPlay={() => setPlaying(true)}
            onPause={() => {
              setPlaying(false);
              if (!whole) pieces.paused();
            }}
            onEnded={() => setPlaying(false)}
            onProgress={() => !whole && pieces.look()}
            onSeeked={() => !whole && pieces.look()}
            onTimeUpdate={(event) => {
              if (!whole) pieces.tick();
              if (playing) onSeek(toFrames(event.currentTarget.currentTime, fps));
            }}
          />
          {!showing && still.data && "png" in still.data && (
            <img src={still.data.png} alt={t.frame} className="size-full object-contain" />
          )}
        </div>
        {/* Shown on hover for a pointer, always on touch. */}
        <Button
          size="icon"
          variant="secondary"
          aria-label={maximized ? t.minimize : t.maximize}
          title={maximized ? t.minimize : t.maximize}
          className="absolute right-2 bottom-2 opacity-0 transition-opacity focus-visible:opacity-100 group-hover:opacity-100 pointer-coarse:opacity-100"
          onClick={maximize}
        >
          {maximized ? <MinimizeIcon /> : <MaximizeIcon />}
        </Button>
      </div>
      {refused ? (
        <p className="text-center text-xs text-destructive">{t.changing}</p>
      ) : (
        said && <p className="text-center text-xs text-muted-foreground">{said}</p>
      )}
      {whole && playable.state === "preparing" && (
        <div className="mx-auto w-full max-w-xs">
          <JobProgressBar job={playable.job} />
        </div>
      )}
      <Transport
        playhead={playhead}
        total={total}
        fps={fps}
        playing={playing}
        quality={quality}
        onSeek={seek}
        onToggle={toggle}
        onQuality={(next) => {
          setQuality(next);
          saveQuality(next);
        }}
      />
    </div>
  );
}
