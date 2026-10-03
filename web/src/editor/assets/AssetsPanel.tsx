// The editor's left sidebar (#702): **Assets**, what this project holds —
// each dragged onto a lane to place it (the desktop app's pool, #543), or
// removed with its bin after a confirm listing the clips that go with it
// (#396) — then **Templates**, inserted at the playhead (#546), and a
// **Library** button whose modal brings a file the user owns into Assets.
// The two lists are kept apart on purpose: what is *in* the project, and what
// is merely *owned* and picked from, as every editor's media bin and import
// dialog are.

import { Trash2Icon } from "lucide-react";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { Button } from "@/components/ui/button";
import type { EditOutcome } from "../project";
import { assetRemoval, confirmThen, showing } from "../removing";
import { TemplatesSection } from "../templates/TemplatesSection";
import { carry } from "./dragged";
import { kindColor, kindName } from "./kinds";
import { LibraryModal } from "./LibraryModal";

interface Props {
  projectId: number;
  document: ProjectDocument;
  edit: EditOutcome;
  playhead: number;
}

export function AssetsPanel({ projectId, document, edit, playhead }: Props) {
  const assets = document.assets ?? [];
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
        <section className="flex flex-col gap-1">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Assets
          </h2>
          {assets.length === 0 && (
            <p className="text-xs text-muted-foreground">
              Nothing yet. Add files from your library, then drag them onto a track.
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
      </div>
      <div className="border-t p-3">
        <LibraryModal projectId={projectId} edit={edit} />
      </div>
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
