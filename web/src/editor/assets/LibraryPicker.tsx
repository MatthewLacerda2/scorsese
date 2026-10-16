// The library page's own browser in picking mode, inside the editor's Library
// modal (#702, `LibraryModal.tsx`): picking a file imports it into the
// project's assets without placing it.

import { useLibrary } from "@/app/queries";
import { FileBrowser, type Picking } from "@/files/FileBrowser";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { importEdit, refusal } from "./picking";

interface Props {
  projectId: number;
  edit: EditOutcome;
}

/** The browser itself — mounted only while the modal is open, so is its query. */
export function LibraryPicker({ projectId, edit }: Props) {
  const t = useT();
  const inProject = useLibrary({ project: projectId });
  const added = new Set((inProject.data ?? []).map((tile) => tile.id));
  const picking: Picking = {
    pick: (tile) => void edit.run(importEdit(tile.id)),
    refuse: (tile) => refusal(tile, added, t.assets.library),
  };
  return (
    <>
      {edit.refused && <p className="text-sm text-destructive">{edit.refused}</p>}
      <FileBrowser picking={picking} />
    </>
  );
}
