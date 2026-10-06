// The editor's left sidebar (#702): **Assets**, what this project holds, as a
// grid of tiles two across (#766, `AssetTile`) — each dragged onto a lane to
// place it (the desktop app's pool, #543), or removed with the bin its tile
// shows on hover, after a confirm listing the clips that go with it (#396) —
// then **Templates**, inserted at the playhead (#546), and a
// **Library** button whose modal brings a file the user owns into Assets.
// The two lists are kept apart on purpose: what is *in* the project, and what
// is merely *owned* and picked from, as every editor's media bin and import
// dialog are.
//
// An image sequence is one tile showing its first still, its stills folded
// under it behind a disclosure arrow (#684) — the desktop panel's grouping,
// from `grouping.ts`. Unfolded, the stills are a row of small pictures across
// the grid, for seeing only: placing and reordering go through the sequence,
// or a sentence to the assistant.

import { useState } from "react";
import type { ProjectDocument } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { TemplatesSection } from "../templates/TemplatesSection";
import { AssetGrid } from "./AssetGrid";
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
          <AssetGrid document={document} edit={edit} unfolded={unfolded} onFold={fold} />
        </section>
        <TemplatesSection edit={edit} playhead={playhead} fps={document.timeline_fps} />
      </div>
      <div className="border-t p-3">
        <LibraryModal projectId={projectId} edit={edit} />
      </div>
    </div>
  );
}
