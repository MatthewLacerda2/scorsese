// "Save as template": the selected clips, named, kept in the user's templates
// (#546) — one `template_save` call, so what a template holds is `core`'s
// answer, the same as when the assistant saves one. A name already taken is
// refused unless "Replace" is ticked, which is how a template is updated.
//
// With nothing selected the button is unavailable, and says why (#1005). Not
// `disabled`: a disabled button takes no pointer events, so its `title` never
// showed. It is `aria-disabled` instead, and its explanation shows on hover,
// on focus, and on a tap where there is no hover.

import { useQueryClient } from "@tanstack/react-query";
import { BookmarkPlusIcon } from "lucide-react";
import { useId, useState } from "react";
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
import type { Messages } from "@/i18n/catalogue";
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

  if (clips.length === 0) return <Unavailable words={words} />;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) edit.dismiss();
      }}
    >
      <DialogTrigger asChild>
        <Button size="sm" variant="outline" title={words.saveTitle} aria-label={words.save}>
          {/* In the header (#764): on a narrow window the icon alone. */}
          <BookmarkPlusIcon /> <span className="hidden lg:inline">{words.save}</span>
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

/** The button while nothing is selected: what a template is, how to make one,
 * and where the ones already made are. */
function Unavailable({ words }: { words: Messages["editor"]["templates"] }) {
  const [tapped, setTapped] = useState(false);
  const id = useId();
  const shown = tapped ? "block" : "hidden group-hover:block group-focus-within:block";
  return (
    <span className="group relative inline-flex">
      <Button
        size="sm"
        variant="outline"
        aria-disabled
        aria-label={words.save}
        aria-describedby={id}
        className="cursor-not-allowed opacity-50"
        onClick={() => setTapped((was) => !was)}
        onBlur={() => setTapped(false)}
      >
        <BookmarkPlusIcon /> <span className="hidden lg:inline">{words.save}</span>
      </Button>
      <span
        id={id}
        role="tooltip"
        className={`absolute top-full right-0 z-50 mt-1 w-64 rounded-md border bg-popover p-2 text-xs text-popover-foreground shadow-md ${shown}`}
      >
        <span className="block">{words.unavailable}</span>
        <span className="mt-1 block text-muted-foreground">{words.where}</span>
      </span>
    </span>
  );
}
