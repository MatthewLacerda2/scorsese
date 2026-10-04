// What a generated clip will be, read from its asset: the sentence a provider
// is asked for (or the recipe synthesis reads), where it is in the sketch
// lifecycle, and the main choices beside it — the desktop inspector's
// shot-brief and still-brief panels (#656), as words.

import type { DocumentAsset } from "@/api";
import type { Messages } from "@/i18n/catalogue";

/** The words a brief is laid out in. */
type Words = Messages["inspector"]["brief"];

/** One generated asset's brief, laid out for reading. */
export interface Brief {
  /** sketch, queued, generated or stale. */
  state: string;
  /** What the brief's text is called: a shot's prompt, a spoken line, a recipe. */
  label: string;
  text: string | undefined;
  /** The main choices the document states, in reading order. Absent ones are
   * the provider's defaults and are not guessed at here. */
  choices: [string, string][];
}

/** The brief of a generated asset, in `t`'s words; `null` for every kind
 * that is not one. */
export function briefOf(asset: DocumentAsset | undefined, t: Words): Brief | null {
  if (!asset) return null;
  const { labels, choices: names } = t;
  const state = asset.state ?? "sketch";
  switch (asset.kind) {
    case "generated_video": {
      const { model, resolution, seconds, aspect } = asset.video ?? {};
      return {
        state,
        label: labels.prompt,
        text: asset.prompt,
        choices: stated([
          [names.model, model],
          [names.size, resolution],
          [names.length, seconds === undefined ? undefined : `${seconds}s`],
          [names.aspect, aspect],
        ]),
      };
    }
    case "generated_image": {
      const { model, resolution, aspect } = asset.image ?? {};
      return {
        state,
        label: labels.prompt,
        text: asset.prompt,
        choices: stated([
          [names.model, model],
          [names.size, resolution],
          [names.aspect, aspect],
        ]),
      };
    }
    case "generated_audio": {
      const { model, voice_id, language } = asset.speech ?? {};
      return {
        state,
        label: labels.line,
        text: asset.prompt,
        choices: stated([
          [names.model, model],
          [names.voice, voice_id],
          [names.language, language],
        ]),
      };
    }
    case "synth_audio":
      return { state, label: labels.recipe, text: recipeName(asset.recipe), choices: [] };
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
