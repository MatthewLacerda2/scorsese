// The editor's controls in the app header (#764): the project's name, its
// platform and style (#1016), Save as template, the frame shape and Render. They used to be a strip of the
// editor's own under the header, which took height the timeline needs while
// the header sat half empty; `HeaderSlot` puts them in that room instead.
//
// On a narrow window the name gives way first — it truncates to nothing
// before a button shrinks — because the name is already on the projects page
// and the buttons are what the user came for. Below `lg` the two buttons drop
// their words and keep their icons (named for a screen reader, and Save as
// template by its hover text too).

import { FilmIcon } from "lucide-react";
import { HeaderSlot } from "@/app/headerSlot";
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
import type { EditOutcome } from "./project";
import { RenderPanel } from "./RenderPanel";
import { StartControl } from "./StartControl";
import { SHAPES, type Shape } from "./shape";
import { SaveTemplate } from "./templates/SaveTemplate";

export interface EditorActionsProps {
  projectId: number;
  name: string;
  /** The selected clips, which Save as template saves. */
  selected: string[];
  edit: EditOutcome;
  shape: Shape;
  onShape: (shape: Shape) => void;
}

/** The editor's controls, drawn in the app header. */
export function EditorHeader(props: EditorActionsProps) {
  return (
    <HeaderSlot>
      <EditorActions {...props} />
    </HeaderSlot>
  );
}

/** What `EditorHeader` puts in the header; its own export so it can be drawn alone. */
export function EditorActions({
  projectId,
  name,
  selected,
  edit,
  shape,
  onShape,
}: EditorActionsProps) {
  const t = useT();
  return (
    <>
      <h1 className="min-w-0 truncate font-heading font-semibold" title={name}>
        {name}
      </h1>
      <div className="ml-auto flex shrink-0 items-center gap-2">
        <StartControl projectId={projectId} onShape={onShape} />
        <SaveTemplate clips={selected} edit={edit} />
        <select
          aria-label={t.editor.page.frameShape}
          className="h-8 rounded-md border bg-transparent px-2 text-sm"
          value={shape}
          onChange={(event) => onShape(event.target.value as Shape)}
        >
          {(Object.keys(SHAPES) as Shape[]).map((choice) => (
            <option key={choice} value={choice}>
              {SHAPES[choice].label}
            </option>
          ))}
        </select>
        <Dialog>
          <DialogTrigger asChild>
            <Button size="sm" variant="outline" aria-label={t.editor.page.render}>
              <FilmIcon /> <span className="hidden lg:inline">{t.editor.page.render}</span>
            </Button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-sm">
            <DialogHeader>
              <DialogTitle>{t.editor.page.renderTitle}</DialogTitle>
              <DialogDescription>{t.editor.page.renderDescription}</DialogDescription>
            </DialogHeader>
            <RenderPanel projectId={projectId} name={name} shape={shape} />
          </DialogContent>
        </Dialog>
      </div>
    </>
  );
}
