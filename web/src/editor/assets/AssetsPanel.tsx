// The editor's left sidebar (#702): **Assets**, what this project holds —
// each dragged onto a lane to place it (the desktop app's pool, #543), or
// removed with its bin after a confirm listing the clips that go with it
// (#396) — then **Templates**, inserted at the playhead (#546), and a
// **Library** button whose modal brings a file the user owns into Assets.
// The two lists are kept apart on purpose: what is *in* the project, and what
// is merely *owned* and picked from, as every editor's media bin and import
// dialog are.
//
// An image sequence is one row, its stills folded under it behind a
// disclosure arrow (#684) — the desktop panel's grouping, from `grouping.ts`.
// Unfolded, the stills are for seeing only: placing and reordering go through
// the sequence, or a sentence to the assistant.

import { ChevronDownIcon, ChevronRightIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import type { DocumentAsset, ProjectDocument } from "@/api";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { assetRemoval, confirmThen, showing } from "../removing";
import { TemplatesSection } from "../templates/TemplatesSection";
import { carry } from "./dragged";
import { grouped } from "./grouping";
import { kindColor, kindName } from "./kinds";
import { LibraryModal } from "./LibraryModal";

interface Props {
  projectId: number;
  document: ProjectDocument;
  edit: EditOutcome;
  playhead: number;
}

export function AssetsPanel({ projectId, document, edit, playhead }: Props) {
  const t = useT();
  const assets = document.assets ?? [];
  const [unfolded, setUnfolded] = useState<ReadonlySet<string>>(new Set());
  const fold = (id: string) =>
    setUnfolded((was) => {
      const now = new Set(was);
      if (!now.delete(id)) now.add(id);
      return now;
    });
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
        <section className="flex flex-col gap-1">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t.assets.heading}
          </h2>
          {assets.length === 0 && <p className="text-xs text-muted-foreground">{t.assets.empty}</p>}
          {grouped(assets).map(({ asset, stills }) => (
            <div key={asset.id} className="flex flex-col gap-1">
              <ProjectAsset
                asset={asset}
                stills={stills.length}
                open={unfolded.has(asset.id)}
                onFold={() => fold(asset.id)}
                uses={showing(document, asset.id).length}
                pending={edit.pending}
                onRemove={() =>
                  confirmThen(assetRemoval(document, asset.id, t.editor.removal), edit.run)
                }
              />
              {unfolded.has(asset.id) && stills.length > 0 && (
                <ul className="ml-6 flex flex-col text-xs text-muted-foreground">
                  {stills.map((still) => (
                    <li key={still.id} className="flex justify-between gap-2 truncate">
                      <span className="truncate">{still.id}</span>
                      {showing(document, still.id).length > 0 && (
                        <span className="shrink-0">{t.assets.alsoAlone}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
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
  /** How many stills are folded under this row; zero for all but a sequence. */
  stills: number;
  open: boolean;
  onFold: () => void;
  uses: number;
  pending: boolean;
  onRemove: () => void;
}

function ProjectAsset({ asset, stills, open, onFold, uses, pending, onRemove }: ProjectAssetProps) {
  const t = useT();
  const name = asset.text ?? asset.id;
  const state = asset.state && (t.assets.state as Record<string, string>)[asset.state];
  return (
    <div className="flex items-center gap-1">
      {stills > 0 && (
        <Button
          size="icon"
          variant="ghost"
          aria-expanded={open}
          aria-label={open ? t.assets.fold(asset.id) : t.assets.unfold(asset.id)}
          onClick={onFold}
        >
          {open ? <ChevronDownIcon /> : <ChevronRightIcon />}
        </Button>
      )}
      <button
        type="button"
        draggable
        onDragStart={(event) =>
          carry(event, { from: "project", asset: asset.id, kind: asset.kind })
        }
        title={t.assets.drag}
        className="flex min-w-0 flex-1 cursor-grab items-center gap-2 rounded-md border px-2 py-1.5 text-left text-sm hover:bg-muted/50 active:cursor-grabbing"
      >
        <span className={`size-2.5 shrink-0 rounded-sm ${kindColor(asset.kind)}`} aria-hidden />
        <span className="min-w-0 flex-1 truncate">
          {stills > 0 ? t.assets.photos(name, stills) : name}
        </span>
        <span className="shrink-0 text-xs text-muted-foreground">
          {kindName(asset.kind, t.assets.kinds)}
          {asset.state && asset.state !== "generated" ? ` · ${state ?? asset.state}` : ""}
          {uses > 0 ? ` · ${uses}×` : ""}
        </span>
      </button>
      <Button
        size="icon"
        variant="ghost"
        aria-label={t.assets.remove(asset.id)}
        title={t.assets.removeTitle}
        disabled={pending}
        onClick={onRemove}
      >
        <Trash2Icon />
      </Button>
    </div>
  );
}
