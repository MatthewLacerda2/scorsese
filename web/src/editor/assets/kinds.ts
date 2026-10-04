// An asset kind's name and colour, the same in the assets panel, on the
// timeline's clips and in the inspector — three panels, one code, as the
// desktop app does it. The names are the catalogue's (`assets.kinds`).

import type { Messages } from "@/i18n/catalogue";

const COLORS: Record<string, string> = {
  video: "bg-sky-500",
  generated_video: "bg-sky-400",
  image: "bg-teal-500",
  generated_image: "bg-teal-400",
  image_sequence: "bg-teal-600",
  audio: "bg-emerald-500",
  generated_audio: "bg-lime-500",
  synth_audio: "bg-green-600",
  text: "bg-amber-500",
  color: "bg-fuchsia-500",
  shape: "bg-violet-500",
  icon: "bg-indigo-500",
  group: "bg-purple-500",
};

/**
 * The kind's name in `names` (the page's `t.assets.kinds`); a kind this build
 * does not know reads as itself.
 */
export function kindName(kind: string, names: Messages["assets"]["kinds"]): string {
  return (names as Record<string, string>)[kind] ?? kind;
}

/** A Tailwind background class for the kind; grey for one this build does not know. */
export function kindColor(kind: string | undefined): string {
  return (kind && COLORS[kind]) || "bg-zinc-500";
}
