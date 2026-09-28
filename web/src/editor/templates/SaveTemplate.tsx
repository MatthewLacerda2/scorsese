// "Save as template": the selected clips, named, kept in the user's templates
// (#546) — one `template_save` call, so what a template holds is `core`'s
// answer, the same as when the assistant saves one. A name already taken is
// refused unless "Replace" is ticked, which is how a template is updated.

import { useQueryClient } from "@tanstack/react-query";
import { BookmarkPlusIcon } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { EditOutcome } from "../project";

export function SaveTemplate({ clips, edit }: { clips: string[]; edit: EditOutcome }) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [replace, setReplace] = useState(false);
  const count = clips.length === 1 ? "1 clip" : `${clips.length} clips`;

  const save = async () => {
    const args = { clips, name: name.trim(), replace };
    const answer = await edit.run({ tool: "template_save", args, edit: false });
    if (!answer) return;
    queryClient.invalidateQueries({ queryKey: ["templates"] });
    setOpen(false);
    setName("");
    setReplace(false);
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) edit.dismiss();
      }}
    >
      <DialogTrigger asChild>
        <Button
          size="sm"
          variant="outline"
          disabled={clips.length === 0}
          title="Select clips on the timeline (Shift-click for several), then save them to reuse"
        >
          <BookmarkPlusIcon /> Save as template
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>Save as template</DialogTitle>
          <DialogDescription>
            {`The ${count} selected, to put into any of your projects later. Changing the template never changes a video it was used in.`}
          </DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <Input
            aria-label="Template name"
            placeholder="Intro, outro, lower third…"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={replace}
              onChange={(event) => setReplace(event.target.checked)}
            />
            Replace the template that has this name
          </label>
          {edit.refused && <p className="text-sm text-destructive">{edit.refused}</p>}
          <DialogFooter>
            <Button type="submit" disabled={!name.trim() || edit.pending}>
              Save
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
