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
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";

export function SaveTemplate({ clips, edit }: { clips: string[]; edit: EditOutcome }) {
  const queryClient = useQueryClient();
  const t = useT();
  const words = t.editor.templates;
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [replace, setReplace] = useState(false);

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
        <Button size="sm" variant="outline" disabled={clips.length === 0} title={words.saveTitle}>
          <BookmarkPlusIcon /> {words.save}
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{words.save}</DialogTitle>
          <DialogDescription>{words.saveDescription(clips.length)}</DialogDescription>
        </DialogHeader>
        <form
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <Input
            aria-label={words.name}
            placeholder={words.placeholder}
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={replace}
              onChange={(event) => setReplace(event.target.checked)}
            />
            {words.replace}
          </label>
          {edit.refused && <p className="text-sm text-destructive">{edit.refused}</p>}
          <DialogFooter>
            <Button type="submit" disabled={!name.trim() || edit.pending}>
              {t.common.save}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
