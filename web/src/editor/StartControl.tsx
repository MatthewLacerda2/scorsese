// The project's platform and style, changeable from the editor (#1016): a
// button in the header naming what the project is made for, and a modal with
// the same two pickers the new-project dialog has. Saving keeps the choice and
// hands it to the assistant, which updates the script and says what changes —
// the turn shows in the chat panel like any other. The frame shape follows
// the platform, as it did when the project was created.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { TargetIcon } from "lucide-react";
import { useState } from "react";
import { api } from "@/api";
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
import { keptStyle, nameOf, shapeOf } from "@/start/menu";
import { PlatformPicker, StylePicker, useStyleMenu } from "@/start/Pickers";
import { editorKey, useEditorProject } from "./project";
import type { Shape } from "./shape";

interface Props {
  projectId: number;
  onShape: (shape: Shape) => void;
}

export function StartControl({ projectId, onShape }: Props) {
  const t = useT().start;
  const project = useEditorProject(projectId);
  const menu = useStyleMenu();
  const [open, setOpen] = useState(false);
  const chosen =
    project.data && menu.data
      ? [
          nameOf(menu.data, "platforms", project.data.platform),
          nameOf(menu.data, "styles", project.data.style),
        ]
          .filter(Boolean)
          .join(" · ")
      : "";
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline" aria-label={t.button} title={chosen || t.button}>
          <TargetIcon />{" "}
          <span className="hidden max-w-48 truncate lg:inline">{chosen || t.button}</span>
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[85vh] flex-col overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{t.changeTitle}</DialogTitle>
          <DialogDescription>{t.changeDescription}</DialogDescription>
        </DialogHeader>
        {open && project.data && menu.data && (
          <Change
            projectId={projectId}
            platform={project.data.platform}
            style={project.data.style}
            onSaved={(platform) => {
              const shape = menu.data && shapeOf(menu.data, platform);
              if (shape) onShape(shape);
              setOpen(false);
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** The pickers, holding the choice until Save. */
function Change({
  projectId,
  platform: was,
  style: wasStyle,
  onSaved,
}: {
  projectId: number;
  platform: string | null;
  style: string | null;
  onSaved: (platform: string | null) => void;
}) {
  const t = useT().start;
  const queryClient = useQueryClient();
  const menu = useStyleMenu();
  const [platform, setPlatform] = useState(was);
  const [style, setStyle] = useState(wasStyle);
  const [note, setNote] = useState<string | null>(null);
  const save = useMutation({
    mutationFn: () =>
      api.projects.retarget(projectId, { platform, style, message: t.changedMessage }),
    onSuccess: (answer) => {
      queryClient.invalidateQueries({ queryKey: editorKey(projectId) });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      if (answer.note) setNote(t.notTold(answer.note));
      else onSaved(platform);
    },
  });
  if (!menu.data) return null;
  const offered = menu.data;
  return (
    <div className="flex flex-col gap-5">
      <PlatformPicker
        menu={offered}
        value={platform}
        onChange={(next) => {
          setPlatform(next);
          setStyle(keptStyle(offered, next, style));
        }}
      />
      <StylePicker menu={offered} platform={platform} value={style} onChange={setStyle} />
      {save.isError && <p className="text-sm text-destructive">{save.error.message}</p>}
      {note && <p className="text-sm text-destructive">{note}</p>}
      <div className="flex justify-end border-t pt-3">
        <Button onClick={() => save.mutate()} disabled={save.isPending}>
          {t.save}
        </Button>
      </div>
    </div>
  );
}
