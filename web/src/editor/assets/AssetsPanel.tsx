// The assets panel: the project's own assets, and the user's library beneath
// them. Either is dragged onto a lane to place it (the desktop app's pool,
// #543); a library file is brought into the project on the way (`import`).
// New files arrive through the library page's uploads, which stay on screen.

import { LibraryIcon } from "lucide-react";
import { Link } from "react-router";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { useLibrary } from "@/app/queries";
import { Thumbnail } from "@/files/FileTile";
import { carry } from "./dragged";
import { kindColor, kindName } from "./kinds";

export function AssetsPanel({ document }: { document: ProjectDocument }) {
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
          <ProjectAsset key={asset.id} asset={asset} uses={uses(document, asset.id)} />
        ))}
      </section>
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
          {library.data?.map((tile) => (
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

function ProjectAsset({ asset, uses }: { asset: DocumentAsset; uses: number }) {
  return (
    <button
      type="button"
      draggable
      onDragStart={(event) => carry(event, { from: "project", asset: asset.id, kind: asset.kind })}
      title="Drag onto a track to place it"
      className="flex cursor-grab items-center gap-2 rounded-md border px-2 py-1.5 text-left text-sm hover:bg-muted/50 active:cursor-grabbing"
    >
      <span className={`size-2.5 shrink-0 rounded-sm ${kindColor(asset.kind)}`} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{asset.text ?? asset.id}</span>
      <span className="shrink-0 text-xs text-muted-foreground">
        {kindName(asset.kind)}
        {asset.state && asset.state !== "generated" ? ` · ${asset.state}` : ""}
        {uses > 0 ? ` · ${uses}×` : ""}
      </span>
    </button>
  );
}

/** How many clips show `asset`. */
function uses(document: ProjectDocument, asset: string): number {
  return (document.tracks ?? []).reduce(
    (count, track) => count + track.clips.filter((clip) => clip.asset === asset).length,
    0,
  );
}
