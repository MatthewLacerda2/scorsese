// The assets section's grid (#766): one tile per top-level asset, two across
// the sidebar's 15rem — wide enough that a thumbnail still reads as a picture,
// which three across would not. An unfolded sequence's stills (#684) take the
// row under its tile, three small pictures across. Kept apart from the panel
// so the grid draws from what is unfolded rather than owning it.

import { Fragment } from "react";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { Thumbnail } from "@/files/FileTile";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { assetRemoval, confirmThen, showing } from "../removing";
import { AssetTile, useThumbnail } from "./AssetTile";
import { grouped } from "./grouping";
import { look } from "./look";

interface Props {
  document: ProjectDocument;
  edit: EditOutcome;
  /** The sequences whose stills are showing. */
  unfolded: ReadonlySet<string>;
  onFold: (id: string) => void;
}

export function AssetGrid({ document, edit, unfolded, onFold }: Props) {
  const t = useT();
  return (
    <div className="grid grid-cols-2 gap-2">
      {grouped(document.assets ?? []).map(({ asset, stills }) => (
        <Fragment key={asset.id}>
          <AssetTile
            asset={asset}
            first={stills[0]}
            stills={stills.length}
            open={unfolded.has(asset.id)}
            onFold={() => onFold(asset.id)}
            pending={edit.pending}
            onRemove={() =>
              confirmThen(assetRemoval(document, asset.id, t.editor.removal), edit.run)
            }
          />
          {unfolded.has(asset.id) && stills.length > 0 && (
            <ul className="col-span-2 grid grid-cols-3 gap-1">
              {stills.map((still) => (
                <Still
                  key={still.id}
                  still={still}
                  alone={showing(document, still.id).length > 0}
                />
              ))}
            </ul>
          )}
        </Fragment>
      ))}
    </div>
  );
}

/** One of an unfolded sequence's stills: its picture, its id on hover, and
 * a mark when a clip also shows it on its own. */
function Still({ still, alone }: { still: DocumentAsset; alone: boolean }) {
  const t = useT();
  const shown = look(still);
  const src = useThumbnail("picture" in shown ? shown.picture.sha256 : undefined);
  const Glyph = shown.glyph;
  return (
    <li
      title={alone ? `${still.id} · ${t.assets.alsoAlone}` : still.id}
      className={`overflow-hidden rounded-sm ${alone ? "ring-2 ring-primary/60" : ""}`}
    >
      {"picture" in shown && src ? (
        <Thumbnail src={src} kind={shown.picture.kind} />
      ) : (
        <div className="flex aspect-video items-center justify-center bg-muted">
          <Glyph className="size-4 text-muted-foreground" aria-hidden />
        </div>
      )}
    </li>
  );
}
