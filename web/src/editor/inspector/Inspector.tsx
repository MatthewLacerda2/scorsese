// The inspector: the selected clip's plain values — start, duration, speed,
// fit, and where its picture sits — with the desktop inspector's rules
// (`app/src/inspector/one.rs`, `transform.rs`). Nothing here edits a
// keyframe: an animated property says so and offers no value, because one
// field cannot hold a ramp and typing over it would flatten somebody's work.
//
// A value is sent when the field is left or Enter is pressed — each change is
// a round trip to the server's tools (`trim_clip`, `clip_set`) — and a refused
// one springs back to what the document still says.

import type { Clip, DocumentAsset, FitMode, Fps, Track } from "@/api";
import { kindColor, kindName } from "../assets/kinds";
import { toSeconds } from "../timeline/time";
import { animated, type Held, shown, stored, transformOf, type Unit } from "./held";
import { NumberField } from "./NumberField";

/** The speeds offered, as an editor's menu offers them. */
const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 4];

interface Props {
  clip: Clip;
  track: Track;
  asset: DocumentAsset | undefined;
  fps: Fps;
  /** Run `trim_clip` or `clip_set` on this clip with these arguments. */
  onChange: (tool: "trim_clip" | "clip_set", args: Record<string, unknown>) => void;
}

export function Inspector({ clip, track, asset, fps, onChange }: Props) {
  const picture = track.kind === "video";
  const transform = transformOf(clip);
  const trim = (field: string) => (value: number) =>
    onChange("trim_clip", { clip: clip.id, [field]: value });
  const set = (field: string) => (value: number | string) =>
    onChange("clip_set", { clip: clip.id, [field]: value });
  const speed = clip.speed ?? 1;
  const ramps = animated(clip);
  return (
    <div className="flex flex-col gap-3 p-3 text-sm">
      <div>
        <div className="flex items-center gap-2">
          <span className={`size-2.5 rounded-sm ${kindColor(asset?.kind)}`} aria-hidden />
          <span className="truncate font-medium">{asset?.text ?? clip.asset}</span>
        </div>
        <p className="text-xs text-muted-foreground">
          {asset ? kindName(asset.kind) : "not in the assets table"}
          {asset?.state && asset.state !== "generated" ? ` · ${asset.state}` : ""} · on track{" "}
          {track.name ?? track.id}
        </p>
      </div>
      <div className="grid grid-cols-[5.5rem_1fr] items-center gap-x-2 gap-y-1.5">
        <span>Start</span>
        <NumberField
          value={toSeconds(clip.start, fps)}
          suffix="s"
          min={0}
          onCommit={trim("start_seconds")}
        />
        <span>Duration</span>
        <NumberField
          value={toSeconds(clip.duration, fps)}
          suffix="s"
          min={0.01}
          onCommit={trim("duration_seconds")}
        />
        <span>Speed</span>
        <select
          className="h-8 rounded-md border bg-transparent px-2"
          value={SPEEDS.includes(speed) ? String(speed) : "other"}
          onChange={(event) => set("speed")(Number(event.target.value))}
        >
          {!SPEEDS.includes(speed) && <option value="other">{speed}×</option>}
          {SPEEDS.map((choice) => (
            <option key={choice} value={choice}>
              {choice}×
            </option>
          ))}
        </select>
        {picture && (
          <>
            <span>Fit</span>
            <select
              className="h-8 rounded-md border bg-transparent px-2"
              value={clip.fit ?? "fit"}
              onChange={(event) => set("fit")(event.target.value as FitMode)}
            >
              <option value="fit">Fit — whole picture</option>
              <option value="fill">Fill — no bars</option>
              <option value="native">Native — own pixel size</option>
            </select>
            <span title="How far from where it naturally sits: right and down, in % of the frame">
              Position
            </span>
            <div className="flex gap-1">
              <Value held={transform.x} unit="percent" prefix="x" onCommit={set("position_x")} />
              <Value held={transform.y} unit="percent" prefix="y" onCommit={set("position_y")} />
            </div>
            <span>Rotation</span>
            <Value held={transform.rotation} unit="degrees" suffix="°" onCommit={set("rotation")} />
            <span>Scale</span>
            <Value
              held={transform.scale}
              unit="percent"
              suffix="%"
              min={1}
              onCommit={set("scale")}
            />
          </>
        )}
      </div>
      {ramps.length > 0 && (
        <div className="flex flex-col gap-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Animated
          </h3>
          {ramps.map((ramp) => (
            <p key={`${ramp.property}-${ramp.by}-${ramp.points}`} className="text-xs">
              {ramp.property}{" "}
              <span className="text-muted-foreground">
                {ramp.points === 1 ? "1 point" : `${ramp.points} points`}
                {ramp.by ? ` · ${ramp.by}` : " · by hand"}
              </span>
            </p>
          ))}
          <p className="text-xs text-muted-foreground">
            Changing an animation is a sentence to the assistant.
          </p>
        </div>
      )}
    </div>
  );
}

/** One transform value: a field when it holds one value, words when it does not. */
function Value(props: {
  held: Held;
  unit: Unit;
  prefix?: string;
  suffix?: string;
  min?: number;
  onCommit: (value: number) => void;
}) {
  const { held, unit } = props;
  if (held.kind === "animated") {
    return (
      <span
        className="text-xs text-muted-foreground italic"
        title="This changes over the clip — nothing here will flatten it"
      >
        animated
      </span>
    );
  }
  if (held.kind === "stretched") {
    return (
      <span
        className="text-xs text-muted-foreground"
        title="Stretched on purpose — ask the assistant for a single value"
      >
        {Math.round(held.wide * 100)}% wide, {Math.round(held.tall * 100)}% tall
      </span>
    );
  }
  return (
    <NumberField
      value={shown(held.value, unit)}
      prefix={props.prefix}
      suffix={props.suffix ?? (unit === "percent" ? "%" : "")}
      min={props.min}
      onCommit={(value) => props.onCommit(stored(value, unit))}
    />
  );
}
