// The row under the preview: start, a frame back, play/pause, a frame
// forward, end, a scrub bar, the clock, and the preview quality, its hover
// text saying what each one means.

import {
  PauseIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StepBackIcon,
  StepForwardIcon,
} from "lucide-react";
import type { Fps } from "@/api";
import { Button } from "@/components/ui/button";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { useT } from "@/i18n/I18nProvider";
import { clock } from "../timeline/time";
import { QUALITIES, type Quality } from "./quality";

interface Props {
  playhead: number;
  total: number;
  fps: Fps;
  playing: boolean;
  quality: Quality;
  onSeek: (frame: number) => void;
  onToggle: () => void;
  onQuality: (quality: Quality) => void;
}

export function Transport({
  playhead,
  total,
  fps,
  playing,
  quality,
  onSeek,
  onToggle,
  onQuality,
}: Props) {
  const t = useT().editor.preview;
  return (
    <div className="flex items-center gap-1">
      <Button size="icon" variant="ghost" aria-label={t.start} onClick={() => onSeek(0)}>
        <SkipBackIcon />
      </Button>
      <Button size="icon" variant="ghost" aria-label={t.back} onClick={() => onSeek(playhead - 1)}>
        <StepBackIcon />
      </Button>
      <Button
        size="icon"
        aria-label={playing ? t.pause : t.play}
        onClick={onToggle}
        disabled={total === 0}
      >
        {playing ? <PauseIcon /> : <PlayIcon />}
      </Button>
      <Button
        size="icon"
        variant="ghost"
        aria-label={t.forward}
        onClick={() => onSeek(playhead + 1)}
      >
        <StepForwardIcon />
      </Button>
      <Button size="icon" variant="ghost" aria-label={t.end} onClick={() => onSeek(total)}>
        <SkipForwardIcon />
      </Button>
      <input
        type="range"
        aria-label={t.scrub}
        className="mx-2 flex-1"
        min={0}
        max={Math.max(1, total)}
        value={Math.min(playhead, Math.max(1, total))}
        onChange={(event) => onSeek(Number(event.target.value))}
      />
      {/* One line however narrow the panel (#1006). */}
      <span className="shrink-0 whitespace-nowrap font-mono text-xs tabular-nums">
        {clock(playhead, fps)} / {clock(total, fps)}
      </span>
      <NativeSelect
        aria-label={t.quality}
        title={t.qualities[quality].says}
        size="sm"
        className="text-xs"
        value={quality}
        onChange={(event) => onQuality(event.target.value as Quality)}
      >
        {(Object.keys(QUALITIES) as Quality[]).map((choice) => (
          <NativeSelectOption key={choice} value={choice}>
            {t.qualities[choice].label}
          </NativeSelectOption>
        ))}
      </NativeSelect>
    </div>
  );
}
