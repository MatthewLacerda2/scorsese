// What a made clip is filled with, by its asset's kind: the asset's thumbnail
// tiled along a video or an image, a sound's waveform over the slice it plays
// (#1572, `Waveform`), the kind's solid hue for everything else.
//
// The picture is the one the assets panel shows (`useThumbnail`, the same
// query), so scrolling, zooming or trimming a clip fetches nothing: the clip
// only moves the tiles it already has.

import type { DocumentAsset } from "@/api";
import { useThumbnail } from "../assets/AssetTile";
import { kindColor } from "../assets/kinds";
import { look } from "../assets/look";
import { type Span, Waveform } from "./Waveform";

/** The kinds whose file is a sound, drawn as its waveform. */
const SOUNDS = new Set(["audio", "generated_audio", "synth_audio"]);

/** A tile is as tall as the clip, so a picture repeats at a steady rhythm. */
const TILE = "auto 100%";

function Tiled(props: { sha256: string; hue: string }) {
  const src = useThumbnail(props.sha256);
  return (
    <span
      className={`pointer-events-none absolute inset-0 ${props.hue}`}
      style={
        src
          ? { backgroundImage: `url("${src}")`, backgroundSize: TILE, backgroundRepeat: "repeat-x" }
          : undefined
      }
      data-tiled={src !== undefined}
      aria-hidden
    />
  );
}

export function ClipFill({ asset, span }: { asset: DocumentAsset | undefined; span?: Span }) {
  const hue = kindColor(asset?.kind);
  if (asset && span && SOUNDS.has(asset.kind)) {
    return <Waveform sha256={asset.sha256} span={span} hue={hue} />;
  }
  const shown = asset && look(asset);
  if (shown && "picture" in shown) {
    return <Tiled sha256={(shown as { picture: { sha256: string } }).picture.sha256} hue={hue} />;
  }
  return <span className={`pointer-events-none absolute inset-0 ${hue}`} aria-hidden />;
}
