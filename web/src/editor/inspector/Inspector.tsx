// The inspector: the selected clip's plain values — start, duration, speed,
// fit, and where its picture sits — with the desktop inspector's rules
// (`app/src/inspector/one.rs`, `transform.rs`). Nothing here edits a
// keyframe: an animated property says so and offers no value, because one
// field cannot hold a ramp and typing over it would flatten somebody's work.
// A generated clip's brief is shown, never edited, for the same kind of
// reason: changing it is the assistant's (`BriefPanel`).
//
// A value is sent when the field is left or Enter is pressed — each change is
// a round trip to the server's tools (`clip_move`, `clip_set`, and `sequence`
// for an image sequence's hold and loop) — and a refused
// one springs back to what the document still says.

import type { Clip, DocumentAsset, FitMode, Fps, Track } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import { kindColor, kindName } from "../assets/kinds";
import { toSeconds } from "../timeline/time";
import { BriefPanel } from "./BriefPanel";
import { briefOf } from "./brief";
import { animated, type Held, shown, stored, transformOf, type Unit } from "./held";
import { NumberField } from "./NumberField";

/** The speeds offered, as an editor's menu offers them. */
const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 4];

interface Props {
  clip: Clip;
  track: Track;
  asset: DocumentAsset | undefined;
  fps: Fps;
  /** Run `clip_move` or `clip_set` on this clip, or `sequence` on its asset. */
  onChange: (tool: "clip_move" | "clip_set" | "sequence", args: Record<string, unknown>) => void;
}

export function Inspector({ clip, track, asset, fps, onChange }: Props) {
  const t = useT();
  const words = t.inspector;
  const picture = track.kind === "video";
  const transform = transformOf(clip);
  const trim = (field: string) => (value: number) =>
    onChange("clip_move", { clip: clip.id, [field]: value });
  const set = (field: string) => (value: number | string) =>
    onChange("clip_set", { clip: clip.id, [field]: value });
  const speed = clip.speed ?? 1;
  const ramps = animated(clip);
  const brief = briefOf(asset, words.brief);
  return (
    <div className="flex flex-col gap-3 p-3 text-sm">
      <div>
        <div className="flex items-center gap-2">
          <span className={`size-2.5 rounded-sm ${kindColor(asset?.kind)}`} aria-hidden />
          <span className="truncate font-medium">{asset?.text ?? clip.asset}</span>
        </div>
        <p className="text-xs text-muted-foreground">
          {asset ? kindName(asset.kind, t.assets.kinds) : words.notInAssets}
          {asset?.state && asset.state !== "generated"
            ? ` · ${t.editor.states[asset.state] ?? asset.state}`
            : ""}
          {` · ${words.onTrack(track.name ?? track.id)}`}
        </p>
      </div>
      <div className="grid grid-cols-[5.5rem_1fr] items-center gap-x-2 gap-y-1.5">
        <span>{words.start}</span>
        <NumberField
          value={toSeconds(clip.start, fps)}
          suffix="s"
          min={0}
          onCommit={trim("start_seconds")}
        />
        <span>{words.duration}</span>
        <NumberField
          value={toSeconds(clip.duration, fps)}
          suffix="s"
          min={0.01}
          onCommit={trim("duration_seconds")}
        />
        <span>{words.speed}</span>
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
            <span>{words.fit}</span>
            <select
              className="h-8 rounded-md border bg-transparent px-2"
              value={clip.fit ?? "fit"}
              onChange={(event) => set("fit")(event.target.value as FitMode)}
            >
              <option value="fit">{words.fits.fit}</option>
              <option value="fill">{words.fits.fill}</option>
              <option value="native">{words.fits.native}</option>
            </select>
            <span title={words.positionTitle}>{words.position}</span>
            <div className="flex gap-1">
              <Value held={transform.x} unit="percent" prefix="x" onCommit={set("position_x")} />
              <Value held={transform.y} unit="percent" prefix="y" onCommit={set("position_y")} />
            </div>
            <span>{words.rotation}</span>
            <Value held={transform.rotation} unit="degrees" suffix="°" onCommit={set("rotation")} />
            <span>{words.scale}</span>
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
      {brief && <BriefPanel brief={brief} />}
      {asset?.kind === "image_sequence" && asset.sequence && (
        <Sequence
          sequence={asset.sequence}
          onChange={(args) => onChange("sequence", { asset: asset.id, ...args })}
        />
      )}
      {ramps.length > 0 && (
        <div className="flex flex-col gap-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {words.animated}
          </h3>
          {ramps.map((ramp) => (
            <p key={`${ramp.property}-${ramp.by}-${ramp.points}`} className="text-xs">
              {ramp.property}{" "}
              <span className="text-muted-foreground">
                {words.points(ramp.points)}
                {` · ${ramp.by || words.byHand}`}
              </span>
            </p>
          ))}
          <p className="text-xs text-muted-foreground">{words.animationNote}</p>
        </div>
      )}
    </div>
  );
}

/** An image sequence's hold and loop — on the asset, so every clip of it changes. */
function Sequence(props: {
  sequence: NonNullable<DocumentAsset["sequence"]>;
  onChange: (args: Record<string, unknown>) => void;
}) {
  const { sequence, onChange } = props;
  const t = useT().inspector.sequence;
  const hold = sequence.hold ?? 1;
  const looping = sequence.loop ?? false;
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t.title}
      </h3>
      <p className="text-xs text-muted-foreground">
        {t.count(sequence.stills.length, hold, sequence.stills.length * hold)}
      </p>
      <div className="grid grid-cols-[5.5rem_1fr] items-center gap-x-2 gap-y-1.5">
        <span title={t.holdTitle}>{t.hold}</span>
        <NumberField
          value={hold}
          suffix="f"
          min={1}
          onCommit={(value) => onChange({ hold: Math.round(value) })}
        />
        <span title={t.loopTitle}>{t.loop}</span>
        <input
          type="checkbox"
          className="size-4 justify-self-start"
          checked={looping}
          onChange={(event) => onChange({ loop: event.target.checked })}
        />
      </div>
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
  const t = useT().inspector;
  if (held.kind === "animated") {
    return (
      <span className="text-xs text-muted-foreground italic" title={t.animatedTitle}>
        {t.animatedValue}
      </span>
    );
  }
  if (held.kind === "stretched") {
    return (
      <span className="text-xs text-muted-foreground" title={t.stretchedTitle}>
        {t.stretched(Math.round(held.wide * 100), Math.round(held.tall * 100))}
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
