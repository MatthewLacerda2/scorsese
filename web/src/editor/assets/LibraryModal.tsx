// The Library button under the editor's sidebar, and the modal it opens
// (#702): the library page's own browser in picking mode, over the editor
// with the page behind it blurred. Picking a file imports it into the
// project's assets without placing it; closing (Esc, the cross, a click
// outside) leaves the editor exactly as it was. Uploading works here too, so
// a file the user does not have yet is one place away, not a page away.

import { LibraryIcon } from "lucide-react";
import { useState } from "react";
import { useLibrary } from "@/app/queries";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { FileBrowser, type Picking } from "@/files/FileBrowser";
import type { EditOutcome } from "../project";
import { importEdit, PICKABLE_KINDS, refusal } from "./picking";

interface Props {
  projectId: number;
  edit: EditOutcome;
}

export function LibraryModal({ projectId, edit }: Props) {
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="w-full">
          <LibraryIcon /> Library
        </Button>
      </DialogTrigger>
      <DialogContent
        overlayClassName="bg-black/30 backdrop-blur-md"
        className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-5xl"
      >
        <DialogHeader>
          <DialogTitle>Add from your library</DialogTitle>
          <DialogDescription>
            Pick a file to add it to this project's assets, then drag it from there onto a track.
            New files can be uploaded here, or dropped on this window.
          </DialogDescription>
        </DialogHeader>
        {open && <Picker projectId={projectId} edit={edit} />}
      </DialogContent>
    </Dialog>
  );
}

/** The browser itself — mounted only while the modal is open, so is its query. */
export function Picker({ projectId, edit }: Props) {
  const inProject = useLibrary({ project: projectId });
  const added = new Set((inProject.data ?? []).map((tile) => tile.id));
  const picking: Picking = {
    pick: (tile) => void edit.run(importEdit(tile.id)),
    refuse: (tile) => refusal(tile, added),
    kinds: PICKABLE_KINDS,
  };
  return (
    <>
      {edit.refused && <p className="text-sm text-destructive">{edit.refused}</p>}
      <FileBrowser picking={picking} />
    </>
  );
}
