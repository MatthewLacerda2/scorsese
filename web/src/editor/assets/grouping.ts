// A sequence's stills listed under the sequence (#684), the grouping
// `scorsese_core::listing` gives the desktop panel, `scorsese assets` and
// `project_assets`: a 400-photo timelapse is one row, not 401. A still several
// sequences play sits under the first of them; one a clip also shows on its
// own stays there too and says so. Every asset is listed exactly once.

import type { DocumentAsset } from "@/api";

/** One top-level row of the assets list, and the stills folded under it. */
export interface Listed {
  asset: DocumentAsset;
  /** Each still once, in the order the sequence first shows it. */
  stills: DocumentAsset[];
}

/** The assets in table order, each still moved under the sequence that owns it. */
export function grouped(assets: readonly DocumentAsset[]): Listed[] {
  const byId = new Map(assets.map((asset) => [asset.id, asset]));
  const owner = new Map<string, string>();
  for (const asset of assets) {
    for (const still of asset.sequence?.stills ?? []) {
      // Only an image is ever owned, as in core: a broken document naming
      // something else must not make that row disappear.
      if (byId.get(still)?.kind === "image" && !owner.has(still)) owner.set(still, asset.id);
    }
  }
  return assets
    .filter((asset) => !owner.has(asset.id))
    .map((asset) => {
      const mine = new Set(
        (asset.sequence?.stills ?? []).filter((still) => owner.get(still) === asset.id),
      );
      return { asset, stills: [...mine].flatMap((id) => byId.get(id) ?? []) };
    });
}
