// What an `AssetTile` draws, with no hooks of its own (#766): the edge in the
// kind's colour, the picture, the name and any state under it, the fold
// button of a sequence and the hover remove. `AssetTile.tsx` says why each.

import { ChevronDownIcon, ChevronRightIcon, Trash2Icon } from "lucide-react";
import type { FileKind } from "@/api";
import { Button } from "@/components/ui/button";
import { Thumbnail } from "@/files/Thumbnail";
import type { Messages } from "@/i18n/catalogue";
import { Unmade } from "../timeline/Unmade";
import type { TileProps } from "./AssetTile";
import { carry } from "./dragged";
import { kindColor } from "./kinds";
import type { Look } from "./look";

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
      className={`group relative rounded-lg p-px ${kindColor(asset.kind)}`}
      data-asset={asset.id}
    >
      {/* biome-ignore lint/plugin/shadcnControls: a draggable tile whose whole face is the handle; a Button's fixed height and padding would fight the picture */}
      <button
        type="button"
        draggable
        onClick={props.onOpen}
        onDragStart={(event) =>
          carry(event, { from: "project", asset: asset.id, kind: asset.kind })
        }
        title={t.drag}
        className="flex w-full cursor-grab flex-col overflow-hidden rounded-md bg-background text-left active:cursor-grabbing"
      >
        <Picture shown={props.shown} src={props.src} state={asset.state} kind={asset.kind} />
        <span
          className={`w-full truncate px-0.75 pt-0.5 text-xs font-medium ${state ? "" : "pb-0.5"}`}
          title={label}
        >
          {label}
        </span>
        {state && (
          <span className="w-full truncate px-0.75 pb-0.5 text-[11px] text-muted-foreground">
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
