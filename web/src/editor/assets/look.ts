// What an asset's tile in the sidebar shows (#766): its file's thumbnail when
// it has one, the timeline's hatch when it is a brief nobody has made yet, and
// otherwise a glyph for its kind. The same split the timeline makes with
// `isMade`, so a tile and its clips agree on what exists.

import {
  AppWindowIcon,
  AudioWaveformIcon,
  FileIcon,
  FilmIcon,
  ImageIcon,
  ImagesIcon,
  LayersIcon,
  type LucideIcon,
  PaletteIcon,
  ShapesIcon,
  SmileIcon,
  TypeIcon,
} from "lucide-react";
import type { DocumentAsset, FileKind } from "@/api";
import { isMade } from "../timeline/made";

/** A glyph per kind. Sound has no picture, so every audio kind gets the
 * waveform; a kind this build does not know gets a plain file. */
const GLYPHS: Record<string, LucideIcon> = {
  video: FilmIcon,
  generated_video: FilmIcon,
  image: ImageIcon,
  generated_image: ImageIcon,
  image_sequence: ImagesIcon,
  audio: AudioWaveformIcon,
  generated_audio: AudioWaveformIcon,
  synth_audio: AudioWaveformIcon,
  text: TypeIcon,
  html: AppWindowIcon,
  color: PaletteIcon,
  shape: ShapesIcon,
  icon: SmileIcon,
  group: LayersIcon,
};

/** The kinds whose file is a picture the library draws a thumbnail of, and
 * which library kind that file is. */
const PICTURED: Record<string, FileKind> = {
  video: "video",
  generated_video: "video",
  image: "image",
  generated_image: "image",
};

export type Look =
  /** The library's thumbnail of the file with these bytes. */
  | { picture: { sha256: string; kind: FileKind }; glyph: LucideIcon }
  /** A brief not made yet: drawn as its timeline clip is, hatched. */
  | { unmade: true; glyph: LucideIcon }
  | { glyph: LucideIcon };

/** What `asset`'s tile shows. A sequence shows its first still, `first`. */
export function look(asset: DocumentAsset, first?: DocumentAsset): Look {
  const glyph = GLYPHS[asset.kind] ?? FileIcon;
  if (asset.state && !isMade(asset)) return { unmade: true, glyph };
  const source = asset.kind === "image_sequence" ? first : asset;
  const kind = source && PICTURED[source.kind];
  if (source?.sha256 && kind) return { picture: { sha256: source.sha256, kind }, glyph };
  return { glyph };
}
