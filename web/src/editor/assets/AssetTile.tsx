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
import { ChevronDownIcon, ChevronRightIcon, Trash2Icon } from "lucide-react";
import { api, type DocumentAsset, type FileKind } from "@/api";
import { Button } from "@/components/ui/button";
import { Thumbnail } from "@/files/FileTile";
import type { Messages } from "@/i18n/catalogue";
import { useT } from "@/i18n/I18nProvider";
import { Unmade } from "../timeline/Unmade";
import { carry } from "./dragged";
import { kindColor } from "./kinds";
import { type Look, look } from "./look";

export interface TileProps {
  asset: DocumentAsset;
  /** A sequence's first still, whose picture is the sequence's. */
  first?: DocumentAsset;
  /** How many stills are folded under this tile; zero for all but a sequence. */
  stills: number;
  open: boolean;
  onFold: () => void;
  pending: boolean;
  onRemove: () => void;
}

export function AssetTile(props: TileProps) {
  const t = useT();
  const shown = look(props.asset, props.first);
  const src = useThumbnail("picture" in shown ? shown.picture.sha256 : undefined);
  return <TileView {...props} shown={shown} src={src} words={t.assets} />;
}

/** Where the library's thumbnail of the file with these bytes is, once known. */
export function useThumbnail(sha256: string | undefined): string | undefined {
  const query = useQuery({
    queryKey: ["library", "list", { sha256 }],
    queryFn: () => api.library.list({ sha256 }),
    enabled: sha256 !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  return query.data?.[0]?.thumbnail;
}

interface ViewProps extends TileProps {
  shown: Look;
  /** The thumbnail's address, when the library has the file. */
  src: string | undefined;
  words: Messages["assets"];
}

/** The tile itself, with no hooks of its own — what the tests take apart. */
export function TileView(props: ViewProps) {
  const { asset, stills, words: t } = props;
  const name = asset.text ?? asset.id;
  const state =
    asset.state && asset.state !== "generated"
      ? ((t.state as Record<string, string>)[asset.state] ?? asset.state)
      : undefined;
  const label = stills > 0 ? t.photos(name, stills) : name;
  return (
    <div
      className={`group relative rounded-lg p-0.5 ${kindColor(asset.kind)}`}
      data-asset={asset.id}
    >
      <button
        type="button"
        draggable
        onDragStart={(event) =>
          carry(event, { from: "project", asset: asset.id, kind: asset.kind })
        }
        title={t.drag}
        className="flex w-full cursor-grab flex-col overflow-hidden rounded-md bg-background text-left active:cursor-grabbing"
      >
        <Picture shown={props.shown} src={props.src} state={asset.state} kind={asset.kind} />
        <span
          className={`w-full truncate px-1.5 pt-1 text-xs font-medium ${state ? "" : "pb-1"}`}
          title={label}
        >
          {label}
        </span>
        {state && (
          <span className="w-full truncate px-1.5 pb-1 text-[11px] text-muted-foreground">
            {state}
          </span>
        )}
      </button>
      {stills > 0 && (
        <Button
          size="icon-xs"
          variant="secondary"
          aria-expanded={props.open}
          aria-label={props.open ? t.fold(asset.id) : t.unfold(asset.id)}
          onClick={props.onFold}
          className="absolute top-1 left-1"
        >
          {props.open ? <ChevronDownIcon /> : <ChevronRightIcon />}
        </Button>
      )}
      <Button
        size="icon-xs"
        variant="secondary"
        aria-label={t.remove(asset.id)}
        title={t.removeTitle}
        disabled={props.pending}
        onClick={props.onRemove}
        className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
      >
        <Trash2Icon />
      </Button>
    </div>
  );
}

interface PictureProps {
  shown: Look;
  src: string | undefined;
  state: string | undefined;
  kind: string;
}

/** The tile's top: a thumbnail, the timeline's hatch, or the kind's glyph. */
function Picture({ shown, src, state, kind }: PictureProps) {
  if ("picture" in shown && src)
    return <Thumbnail src={src} kind={shown.picture.kind satisfies FileKind} />;
  const Glyph = shown.glyph;
  return (
    <div className="relative flex aspect-video items-center justify-center overflow-hidden bg-muted">
      {"unmade" in shown && <Unmade kind={kind} state={state} />}
      <Glyph className="size-6 text-muted-foreground" aria-hidden />
    </div>
  );
}
