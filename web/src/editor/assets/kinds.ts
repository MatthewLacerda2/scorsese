// An asset kind's name and colour, the same in the assets panel, on the
// timeline's clips and in the inspector — three panels, one code, as the
// desktop app does it. The names are the catalogue's (`assets.kinds`).
// The hues are the `--kind-*` tokens in index.css, fixed rather than themed: a
// kind's colour is its identity, as a chart series' is, and stays the same in
// light and dark.

import type { Messages } from "@/i18n/catalogue";

const COLORS: Record<string, string> = {
  video: "bg-kind-video",
  generated_video: "bg-kind-generated-video",
  image: "bg-kind-image",
  generated_image: "bg-kind-generated-image",
  image_sequence: "bg-kind-image-sequence",
  audio: "bg-kind-audio",
  generated_audio: "bg-kind-generated-audio",
  synth_audio: "bg-kind-synth-audio",
  text: "bg-kind-text",
  html: "bg-kind-html",
  color: "bg-kind-color",
  shape: "bg-kind-shape",
  icon: "bg-kind-icon",
  group: "bg-kind-group",
};

/** The same hues as 1px outlines; spelt out because Tailwind reads whole class names. */
const OUTLINES: Record<string, string> = {
  video: "border-kind-video",
  generated_video: "border-kind-generated-video",
  image: "border-kind-image",
  generated_image: "border-kind-generated-image",
  image_sequence: "border-kind-image-sequence",
  audio: "border-kind-audio",
  generated_audio: "border-kind-generated-audio",
  synth_audio: "border-kind-synth-audio",
  text: "border-kind-text",
  html: "border-kind-html",
  color: "border-kind-color",
  shape: "border-kind-shape",
  icon: "border-kind-icon",
  group: "border-kind-group",
};

/**
 * The kind's name in `names` (the page's `t.assets.kinds`); a kind this build
 * does not know reads as itself.
 */
export function kindName(kind: string, names: Messages["assets"]["kinds"]): string {
  return (names as Record<string, string>)[kind] ?? kind;
}

/** A theme background class for the kind; grey for one this build does not know. */
export function kindColor(kind: string | undefined): string {
  return (kind && COLORS[kind]) || "bg-kind-other";
}

/** A theme border-colour class for the kind; grey for one this build does not know. */
export function kindOutline(kind: string | undefined): string {
  return (kind && OUTLINES[kind]) || "border-kind-other";
}
