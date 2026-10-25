// One of the project's assets as a tile in the sidebar's grid (#766): its
// picture, its name under it, and a thin edge in its kind's colour — the
// colour its clips have on the timeline, so an asset and its clips still pair
// up at a glance. The edge and the picture say the kind, so the words do not
// (#1006); a state other than `generated` ("sketch") is written under the
// name, since that is money waiting to be spent. Dragged onto a lane it places a clip, as a row did; removing
// is a hover action in the corner, as the library page's tiles carry theirs.
//
// The picture is the library's own thumbnail of the file with the asset's
// bytes: a stored project's file *is* a library item (`library/mod.rs`), found
// by its hash with the same `?sha256=` question an upload asks.

import { useQuery } from "@tanstack/react-query";
import { api, type DocumentAsset, type LibraryTile } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useT } from "@/i18n/I18nProvider";
import { look } from "./look";
import { TileView } from "./TileView";

export interface TileProps {
  asset: DocumentAsset;
  /** A sequence's first still, whose picture is the sequence's. */
  first?: DocumentAsset;
  /** How many stills are folded under this tile; zero for all but a sequence. */
  stills: number;
  open: boolean;
  onFold: () => void;
  /** A click on the tile: show the asset in the side panel. A drag places it instead. */
  onOpen: () => void;
  pending: boolean;
  onRemove: () => void;
}

export function AssetTile(props: TileProps) {
  const t = useT();
  const shown = look(props.asset, props.first);
  const src = useThumbnail("picture" in shown ? shown.picture.sha256 : undefined);
  return <TileView {...props} shown={shown} src={src} words={t.assets} />;
}

/** The library's tile of the file with these bytes, once known. */
export function useLibraryTile(sha256: string | undefined): LibraryTile | undefined {
  const query = useQuery({
    queryKey: queryKeys.libraryList({ sha256 }),
    queryFn: () => api.library.list({ sha256 }),
    enabled: sha256 !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  return query.data?.[0];
}

/** Where the library's thumbnail of the file with these bytes is, once known. */
export function useThumbnail(sha256: string | undefined): string | undefined {
  return useLibraryTile(sha256)?.thumbnail;
}
