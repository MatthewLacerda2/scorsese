// The preview: the frame under the playhead while editing, and the cut
// playing when asked to — with the transport under it (start, a frame back,
// play/pause, a frame forward, end, and a scrub bar).
//
// **Two pictures, one rule: the server draws both.** Until the preview video
// of this revision is ready, the frame under the playhead is a still from the
// `still` tool; then it is the video (`playable.ts`, #542's doctrine), where
// scrubbing is seeking. Both are drawn at the chosen preview quality
// (`quality.ts`), which the row under the picture offers and says.
// Real-time compositing in the browser is not attempted.

import {
  PauseIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StepBackIcon,
  StepForwardIcon,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ProjectDocument } from "@/api";
import { Button } from "@/components/ui/button";
import { JobProgressBar } from "../JobProgressBar";
import { timecode, toFrames, toSeconds } from "../timeline/time";
import { usePlayable, waiting } from "./playable";
import { previewRaster, QUALITIES, type Quality, savedQuality, saveQuality } from "./quality";
import { useStill } from "./still";

interface Props {
  projectId: number;
  revision: number;
  document: ProjectDocument;
  playhead: number;
  onSeek: (frame: number) => void;
  /** The size the film is delivered at, `WIDTHxHEIGHT`: what the preview is a fraction of. */
  deliver: string;
}

/** Where the cut ends: the last picture, since picture decides a render's length. */
export function cutLength(document: ProjectDocument): number {
  const tracks = document.tracks ?? [];
  const ends = (kind?: string) =>
    tracks
      .filter((track) => !kind || track.kind === kind)
      .flatMap((track) => track.clips.map((clip) => clip.start + clip.duration));
  return Math.max(0, ...(ends("video").length ? ends("video") : ends()));
}

export function Preview({ projectId, revision, document, playhead, onSeek, deliver }: Props) {
  const fps = document.timeline_fps;
  const total = cutLength(document);
  const [quality, setQuality] = useState<Quality>(savedQuality);
  const raster = previewRaster(deliver, quality);
  const { playable, ask } = usePlayable(projectId, revision, deliver, quality, total > 0);
  const ready = playable.state === "ready";
  const still = useStill(projectId, revision, playhead, raster, !ready && total > 0);
  const video = useRef<HTMLVideoElement>(null);
  const [playing, setPlaying] = useState(false);
  const [wanted, setWanted] = useState(false);
  const [width, height] = raster.split("x").map(Number);

  // Play was pressed before the render was there: start it once it is.
  useEffect(() => {
    const element = video.current;
    if (!ready || !wanted || !element) return;
    setWanted(false);
    element.currentTime = toSeconds(playhead, fps);
    void element.play();
  }, [ready, wanted, playhead, fps]);
  // Scrubbing a loaded video is seeking it.
  useEffect(() => {
    const element = video.current;
    if (!element || playing) return;
    const at = toSeconds(playhead, fps);
    if (Math.abs(element.currentTime - at) > 0.5 / (fps.num / fps.den)) element.currentTime = at;
  }, [playhead, playing, fps]);

  const seek = (frame: number) => onSeek(Math.min(Math.max(0, frame), total));
  const toggle = () => {
    if (playing) return video.current?.pause();
    if (ready) return void video.current?.play();
    setWanted(true);
    void ask();
  };
  const said =
    playable.state === "preparing"
      ? waiting(playable.job)
      : playable.state === "failed"
        ? `The preview could not be rendered: ${playable.why}`
        : still.data && "none" in still.data
          ? still.data.none
          : total === 0
            ? "Nothing on the timeline yet."
            : null;

  return (
    <div className="flex h-full min-h-0 flex-col gap-2 p-3">
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <div
          className="relative max-h-full max-w-full overflow-hidden rounded-md bg-black"
          style={{ aspectRatio: `${width} / ${height}`, height: "100%" }}
        >
          {ready ? (
            // biome-ignore lint/a11y/useMediaCaption: a preview of the user's own cut has no captions to offer
            <video
              ref={video}
              src={playable.render.file}
              className="size-full object-contain"
              preload="auto"
              onPlay={() => setPlaying(true)}
              onPause={() => setPlaying(false)}
              onEnded={() => setPlaying(false)}
              onTimeUpdate={(event) => {
                if (playing) onSeek(toFrames(event.currentTarget.currentTime, fps));
              }}
            />
          ) : (
            still.data &&
            "png" in still.data && (
              <img
                src={still.data.png}
                alt="The frame under the playhead"
                className="size-full object-contain"
              />
            )
          )}
        </div>
      </div>
      {said && <p className="text-center text-xs text-muted-foreground">{said}</p>}
      {playable.state === "preparing" && (
        <div className="mx-auto w-full max-w-xs">
          <JobProgressBar job={playable.job} />
        </div>
      )}
      <div className="flex items-center gap-1">
        <Button size="icon" variant="ghost" aria-label="To the start" onClick={() => seek(0)}>
          <SkipBackIcon />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          aria-label="One frame back"
          onClick={() => seek(playhead - 1)}
        >
          <StepBackIcon />
        </Button>
        <Button
          size="icon"
          aria-label={playing ? "Pause" : "Play"}
          onClick={toggle}
          disabled={total === 0}
        >
          {playing ? <PauseIcon /> : <PlayIcon />}
        </Button>
        <Button
          size="icon"
          variant="ghost"
          aria-label="One frame forward"
          onClick={() => seek(playhead + 1)}
        >
          <StepForwardIcon />
        </Button>
        <Button size="icon" variant="ghost" aria-label="To the end" onClick={() => seek(total)}>
          <SkipForwardIcon />
        </Button>
        <input
          type="range"
          aria-label="Scrub"
          className="mx-2 flex-1"
          min={0}
          max={Math.max(1, total)}
          value={Math.min(playhead, Math.max(1, total))}
          onChange={(event) => seek(Number(event.target.value))}
        />
        <span className="w-28 text-right font-mono text-xs tabular-nums">
          {timecode(playhead, fps)} / {timecode(total, fps)}
        </span>
        <select
          aria-label="Preview quality"
          title={QUALITIES[quality].says}
          className="h-8 rounded-md border bg-transparent px-1 text-xs"
          value={quality}
          onChange={(event) => {
            const next = event.target.value as Quality;
            setQuality(next);
            saveQuality(next);
          }}
        >
          {(Object.keys(QUALITIES) as Quality[]).map((choice) => (
            <option key={choice} value={choice}>
              {QUALITIES[choice].label}
            </option>
          ))}
        </select>
      </div>
      <p className="text-center text-[11px] text-muted-foreground">
        {`Preview at ${QUALITIES[quality].says} (${raster})`}
      </p>
    </div>
  );
}
