// The editor's controls in the app header (#764): the project's name, its
// platform and style (#1016), the frame shape and Render. They used to be a strip of the
// editor's own under the header, which took height the timeline needs while
// the header sat half empty; `EditorHeader` puts them in that room instead.
//
// On a narrow window the name gives way first — it truncates to nothing
// before a button shrinks — because the name is already on the projects page
// and the buttons are what the user came for. Below `lg` the buttons drop
// their words and keep their icons (named for a screen reader).

import { FilmIcon } from "lucide-react";
import type { Fps } from "@/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "./project";
import { RenderPanel } from "./RenderPanel";
import { StartControl } from "./StartControl";
import { offered } from "./shape";

export interface EditorActionsProps {
  projectId: number;
  name: string;
  edit: EditOutcome;
  /** The project's frame shape, `across:down` (`shape.ts`). */
  aspect: string;
  /** Asks for the frame in another shape: a re-fit, never a setting kept aside. */
  onAspect: (aspect: string) => void;
  /** The project's own rate, which Render offers first. */
  fps: Fps;
}

/** What `EditorHeader` puts in the header; its own file so it can be drawn alone. */
export function EditorActions({
  projectId,
  name,
  edit,
  aspect,
  onAspect,
  fps,
}: EditorActionsProps) {
  const t = useT();
  return (
    <>
      <h1 className="min-w-0 truncate font-heading font-semibold" title={name}>
        {name}
      </h1>
      <div className="ml-auto flex shrink-0 items-center gap-2">
        <StartControl projectId={projectId} onAspect={onAspect} />
        <NativeSelect
          aria-label={t.editor.page.frameShape}
          value={aspect}
          disabled={edit.pending}
          onChange={(event) => onAspect(event.target.value)}
        >
          {offered(aspect).map((choice) => (
            <NativeSelectOption key={choice} value={choice}>
              {choice}
            </NativeSelectOption>
          ))}
        </NativeSelect>
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
            <RenderPanel projectId={projectId} name={name} aspect={aspect} fps={fps} />
          </DialogContent>
        </Dialog>
      </div>
    </>
  );
}
