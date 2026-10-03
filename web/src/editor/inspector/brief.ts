// What a generated clip will be, read from its asset: the sentence a provider
// is asked for (or the recipe synthesis reads), where it is in the sketch
// lifecycle, and the main choices beside it — the desktop inspector's
// shot-brief and still-brief panels (#656), as words.

import type { DocumentAsset } from "@/api";

/** One generated asset's brief, laid out for reading. */
export interface Brief {
  /** sketch, queued, generated or stale. */
  state: string;
  /** What the brief's text is called: a shot's prompt, a spoken line, a recipe. */
  label: "Prompt" | "Line" | "Recipe";
  text: string | undefined;
  /** The main choices the document states, in reading order. Absent ones are
   * the provider's defaults and are not guessed at here. */
  choices: [string, string][];
}

/** What each lifecycle state means to somebody about to press GO. */
export const STATES: Record<string, string> = {
  sketch: "not made yet — previews as a slug card, at no cost",
  queued: "being made",
  generated: "made",
  stale: "the brief changed since it was made — previews as a slug card until it is made again",
};

/** The brief of a generated asset; `null` for every kind that is not one. */
export function briefOf(asset: DocumentAsset | undefined): Brief | null {
  if (!asset) return null;
  const state = asset.state ?? "sketch";
  switch (asset.kind) {
    case "generated_video": {
      const { model, resolution, seconds, aspect } = asset.video ?? {};
      return {
        state,
        label: "Prompt",
        text: asset.prompt,
        choices: stated([
          ["Model", model],
          ["Size", resolution],
          ["Length", seconds === undefined ? undefined : `${seconds}s`],
          ["Aspect", aspect],
        ]),
      };
    }
    case "generated_image": {
      const { model, resolution, aspect } = asset.image ?? {};
      return {
        state,
        label: "Prompt",
        text: asset.prompt,
        choices: stated([
          ["Model", model],
          ["Size", resolution],
          ["Aspect", aspect],
        ]),
      };
    }
    case "generated_audio": {
      const { model, voice_id, language } = asset.speech ?? {};
      return {
        state,
        label: "Line",
        text: asset.prompt,
        choices: stated([
          ["Model", model],
          ["Voice", voice_id],
          ["Language", language],
        ]),
      };
    }
    case "synth_audio":
      return { state, label: "Recipe", text: recipeName(asset.recipe), choices: [] };
    default:
      return null;
  }
}

/** `recipes/rain.json` → `rain`: the name an agent gave the document. */
function recipeName(path: string | undefined): string | undefined {
  return path
    ?.split("/")
    .pop()
    ?.replace(/\.json$/, "");
}

function stated(rows: [string, string | undefined][]): [string, string][] {
  return rows.filter((row): row is [string, string] => row[1] !== undefined);
}
