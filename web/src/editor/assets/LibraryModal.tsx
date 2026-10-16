// The Library button under the editor's sidebar, and the modal it opens
// (#702): the library page's own browser in picking mode, over the editor
// with the page behind it blurred. Picking a file imports it into the
// project's assets without placing it; closing (Esc, the cross, a click
// outside) leaves the editor exactly as it was. Uploading works here too, so
// a file the user does not have yet is one place away, not a page away.

import { LibraryIcon } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { LibraryPicker } from "./LibraryPicker";

interface Props {
  projectId: number;
  edit: EditOutcome;
}

export function LibraryModal({ projectId, edit }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="w-full">
          <LibraryIcon /> {t.assets.library.button}
        </Button>
      </DialogTrigger>
      <DialogContent
        overlayClassName="bg-black/30 backdrop-blur-md"
        className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-5xl"
      >
        <DialogHeader>
          <DialogTitle>{t.assets.library.title}</DialogTitle>
          <DialogDescription>{t.assets.library.description}</DialogDescription>
        </DialogHeader>
        {open && <LibraryPicker projectId={projectId} edit={edit} />}
      </DialogContent>
    </Dialog>
  );
}
