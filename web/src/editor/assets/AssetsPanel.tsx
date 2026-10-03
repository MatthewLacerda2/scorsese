// The assets panel: the project's own assets, the user's templates, and the
// user's library beneath them. An asset or a library file is dragged onto a
// lane to place it (the desktop app's pool, #543); a library file is brought
// into the project on the way (`import`), and a template goes in at the
// playhead (#546). New files arrive through the library page's uploads, which
// stay on screen. An asset's bin button removes it, after a confirm listing
// the clips that go with it (#396). A MIDI file is not listed here: it is notes,
// not media a track can hold, and the assistant reads it into a song (#678).

import { LibraryIcon, Trash2Icon } from "lucide-react";
import { Link } from "react-router";
import type { DocumentAsset, LibraryTile, ProjectDocument } from "@/api";
import { useLibrary } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { Thumbnail } from "@/files/FileTile";
import type { EditOutcome } from "../project";
import { assetRemoval, confirmThen, showing } from "../removing";
import { TemplatesSection } from "../templates/TemplatesSection";
import { carry } from "./dragged";
import { kindColor, kindName } from "./kinds";

/** Whether a library file can be dragged onto a track. */
function placeable(tile: LibraryTile): boolean {
  return tile.kind !== "midi";
}

interface Props {
  document: ProjectDocument;
  edit: EditOutcome;
  playhead: number;
}

export function AssetsPanel({ document, edit, playhead }: Props) {
  const library = useLibrary({});
  const assets = document.assets ?? [];
  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto p-3">
      <section className="flex flex-col gap-1">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          In this project
        </h2>
        {assets.length === 0 && (
          <p className="text-xs text-muted-foreground">
            Nothing yet. Drag a file from your library below onto a track.
          </p>
        )}
        {assets.map((asset) => (
          <ProjectAsset
            key={asset.id}
            asset={asset}
            uses={showing(document, asset.id).length}
            pending={edit.pending}
            onRemove={() => confirmThen(assetRemoval(document, asset.id), edit.run)}
          />
        ))}
      </section>
      <TemplatesSection edit={edit} playhead={playhead} fps={document.timeline_fps} />
      <section className="flex flex-col gap-2">
        <h2 className="flex items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Your library
          <Link to="/library" className="flex items-center gap-1 normal-case hover:text-foreground">
            <LibraryIcon className="size-3" /> Upload
          </Link>
        </h2>
        {library.isError && <p className="text-xs text-destructive">{library.error.message}</p>}
        {library.data?.length === 0 && (
          <p className="text-xs text-muted-foreground">
            Your library is empty — upload files there.
          </p>
        )}
        <div className="grid grid-cols-2 gap-2">
          {library.data?.filter(placeable).map((tile) => (
            <button
              type="button"
              key={tile.id}
              draggable
              onDragStart={(event) =>
                carry(event, { from: "library", item: tile.id, kind: tile.kind })
              }
              title={`${tile.name} — drag onto a track`}
              className="cursor-grab overflow-hidden rounded-md border text-left text-xs active:cursor-grabbing"
            >
              <Thumbnail src={tile.thumbnail} kind={tile.kind} />
              <p className="truncate px-1.5 py-1">{tile.name}</p>
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

interface ProjectAssetProps {
  asset: DocumentAsset;
  uses: number;
  pending: boolean;
  onRemove: () => void;
}

function ProjectAsset({ asset, uses, pending, onRemove }: ProjectAssetProps) {
  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        draggable
        onDragStart={(event) =>
          carry(event, { from: "project", asset: asset.id, kind: asset.kind })
        }
        title="Drag onto a track to place it"
        className="flex min-w-0 flex-1 cursor-grab items-center gap-2 rounded-md border px-2 py-1.5 text-left text-sm hover:bg-muted/50 active:cursor-grabbing"
      >
        <span className={`size-2.5 shrink-0 rounded-sm ${kindColor(asset.kind)}`} aria-hidden />
        <span className="min-w-0 flex-1 truncate">{asset.text ?? asset.id}</span>
        <span className="shrink-0 text-xs text-muted-foreground">
          {kindName(asset.kind)}
          {asset.state && asset.state !== "generated" ? ` · ${asset.state}` : ""}
          {uses > 0 ? ` · ${uses}×` : ""}
        </span>
      </button>
      <Button
        size="icon"
        variant="ghost"
        aria-label={`Remove ${asset.id}`}
        title="Remove from the project — the clips using it go too"
        disabled={pending}
        onClick={onRemove}
      >
        <Trash2Icon />
      </Button>
    </div>
  );
}
