// Whether a clip's asset exists yet — the desktop app's rule
// (`scorsese_core::Asset::has_renderable_media`), so the two timelines hatch
// the same clips. A clip that is not made renders as a slug card, and which
// ones those are is the question asked before pressing the button that spends
// money.

import type { DocumentAsset } from "@/api";

/** The kinds held whole in the document, with no file to wait for. */
const INLINE = new Set(["text", "color", "shape", "icon", "group", "image_sequence"]);

/** True when the clip's asset has something to play: a generated brief with
 * its file, or any other asset with its media. A missing asset is not made. */
export function isMade(asset: DocumentAsset | undefined): boolean {
  if (!asset) return false;
  if (asset.state) return asset.state === "generated" && asset.path !== undefined;
  return asset.path !== undefined || INLINE.has(asset.kind);
}
