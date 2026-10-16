// Creating a project (#1016): Create opens a modal in four steps — the name,
// files from the library, the platform, the style — of which only the name is
// needed. Create works from any step, so someone with a clear idea names the
// project and goes; the rest makes the assistant's first answer aimed.
//
// The style step shows only the styles made for the platform chosen, and a
// change of platform drops a style not made for it, so the server is never
// sent a pair it refuses. The editor opens in the platform's frame shape.

import { PlusIcon } from "lucide-react";
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
import { Wizard } from "./Wizard";

/** The Create button at the head of the projects page, and the modal it opens. */
export function NewProjectButton() {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <PlusIcon /> {t.pages.projects.create}
        </Button>
      </DialogTrigger>
      <DialogContent
        overlayClassName="bg-black/30 backdrop-blur-md"
        className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-4xl"
      >
        <DialogHeader>
          <DialogTitle>{t.start.title}</DialogTitle>
          <DialogDescription>{t.start.description}</DialogDescription>
        </DialogHeader>
        {open && <Wizard />}
      </DialogContent>
    </Dialog>
  );
}
