// An asset kind's name and colour, the same in the assets panel, on the
// timeline's clips and in the inspector — three panels, one code, as the
// desktop app does it.

const NAMES: Record<string, string> = {
  video: "video",
  image: "image",
  audio: "audio",
  text: "text",
  color: "colour",
  shape: "shape",
  icon: "icon",
  group: "group",
  generated_video: "generated video",
  generated_image: "generated still",
  generated_audio: "generated speech",
  synth_audio: "synthesised audio",
};

const COLORS: Record<string, string> = {
  video: "bg-sky-500",
  generated_video: "bg-sky-400",
  image: "bg-teal-500",
  generated_image: "bg-teal-400",
  audio: "bg-emerald-500",
  generated_audio: "bg-lime-500",
  synth_audio: "bg-green-600",
  text: "bg-amber-500",
  color: "bg-fuchsia-500",
  shape: "bg-violet-500",
  icon: "bg-indigo-500",
  group: "bg-purple-500",
};

export function kindName(kind: string): string {
  return NAMES[kind] ?? kind;
}

/** A Tailwind background class for the kind; grey for one this build does not know. */
export function kindColor(kind: string | undefined): string {
  return (kind && COLORS[kind]) || "bg-zinc-500";
}
