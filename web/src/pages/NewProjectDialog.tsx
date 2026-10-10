// Creating a project (#1016): Create opens a modal in four steps — the name,
// files from the library, the platform, the style — of which only the name is
// needed. Create works from any step, so someone with a clear idea names the
// project and goes; the rest makes the assistant's first answer aimed.
//
// The style step shows only the styles made for the platform chosen, and a
// change of platform drops a style not made for it, so the server is never
// sent a pair it refuses. The editor opens in the platform's frame shape.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { cn } from "cn";
import { PlusIcon } from "lucide-react";
import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router";
import { api, type NewProject, type StyleMenu } from "@/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { saveShape } from "@/editor/shape";
import { FileBrowser } from "@/files/FileBrowser";
import { useT } from "@/i18n/I18nProvider";
import { keptStyle, shapeOf } from "@/start/menu";
import { PlatformPicker, StylePicker, useStyleMenu } from "@/start/Pickers";

export const STEPS = ["name", "assets", "platform", "style"] as const;
export type Step = (typeof STEPS)[number];

/** What the dialog holds until Create. */
export interface Draft {
  name: string;
  assets: number[];
  platform: string | null;
  style: string | null;
}

export const EMPTY: Draft = { name: "", assets: [], platform: null, style: null };

/** The request a draft makes — or `null` when it has no name yet, and creates nothing. */
export function bodyOf(draft: Draft): NewProject | null {
  const name = draft.name.trim();
  if (!name) return null;
  return { name, platform: draft.platform, style: draft.style, assets: draft.assets };
}

/** The draft with `id` picked, or unpicked when it already was. */
export function toggled(draft: Draft, id: number): Draft {
  const assets = draft.assets.includes(id)
    ? draft.assets.filter((picked) => picked !== id)
    : [...draft.assets, id];
  return { ...draft, assets };
}

/** The draft on `platform`, keeping its style only when it is made for that. */
export function onPlatform(draft: Draft, menu: StyleMenu, platform: string | null): Draft {
  return { ...draft, platform, style: keptStyle(menu, platform, draft.style) };
}

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

/** The steps — mounted only while the modal is open, so a new one starts blank. */
export function Wizard({ initial = EMPTY }: { initial?: Draft }) {
  const t = useT().start;
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const menu = useStyleMenu();
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState(initial);
  const [missing, setMissing] = useState(false);
  const create = useMutation({
    mutationFn: api.projects.create,
    onSuccess: (project) => {
      const shape = menu.data ? shapeOf(menu.data, draft.platform) : null;
      if (shape) saveShape(project.id, shape);
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["library"] });
      navigate(`/projects/${project.id}/edit`);
    },
  });
  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    if (create.isPending) return;
    const body = bodyOf(draft);
    if (!body) {
      setMissing(true);
      setStep(0);
      return;
    }
    create.mutate(body);
  };
  const current = STEPS[step] ?? "name";
  return (
    <div className="flex flex-col gap-4">
      <ol className="flex flex-wrap gap-2 text-sm">
        {STEPS.map((name, index) => (
          <li key={name}>
            <button
              type="button"
              aria-current={index === step ? "step" : undefined}
              onClick={() => setStep(index)}
              className={cn(
                "rounded-full border px-3 py-1",
                index === step
                  ? "border-primary bg-primary/10 font-medium"
                  : "text-muted-foreground",
              )}
            >
              {index + 1}. {t.steps[name]}
            </button>
          </li>
        ))}
      </ol>
      {current === "name" && (
        <form onSubmit={submit} className="flex flex-col gap-2">
          <Input
            aria-label={t.name}
            placeholder={t.name}
            value={draft.name}
            autoFocus
            aria-invalid={missing || undefined}
            aria-describedby={missing ? "new-project-missing" : undefined}
            onChange={(event) => {
              setDraft({ ...draft, name: event.target.value });
              setMissing(false);
            }}
          />
          {missing && (
            <p id="new-project-missing" className="text-sm text-destructive">
              {t.nameMissing}
            </p>
          )}
        </form>
      )}
      {current === "assets" && (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-muted-foreground">
            {t.assetsHint} {draft.assets.length > 0 && t.picked(draft.assets.length)}
          </p>
          <FileBrowser
            picking={{
              pick: (tile) => setDraft((was) => toggled(was, tile.id)),
              refuse: () => null,
              chosen: (tile) => draft.assets.includes(tile.id),
            }}
          />
        </div>
      )}
      {current === "platform" && menu.data && (
        <PlatformPicker
          menu={menu.data}
          value={draft.platform}
          onChange={(platform) => menu.data && setDraft(onPlatform(draft, menu.data, platform))}
        />
      )}
      {current === "style" && menu.data && (
        <StylePicker
          menu={menu.data}
          platform={draft.platform}
          value={draft.style}
          onChange={(style) => setDraft({ ...draft, style })}
        />
      )}
      {menu.isError && current !== "name" && current !== "assets" && (
        <p className="text-sm text-destructive">{menu.error.message}</p>
      )}
      {create.isError && <p className="text-sm text-destructive">{create.error.message}</p>}
      <div className="flex items-center gap-2 border-t pt-3">
        <span className="text-xs text-muted-foreground">{t.stepOf(step + 1, STEPS.length)}</span>
        <div className="ml-auto flex gap-2">
          {step > 0 && (
            <Button variant="ghost" onClick={() => setStep(step - 1)}>
              {t.back}
            </Button>
          )}
          {step < STEPS.length - 1 && (
            <Button variant="outline" onClick={() => setStep(step + 1)}>
              {t.next}
            </Button>
          )}
          <Button onClick={() => submit()} disabled={create.isPending}>
            <PlusIcon /> {t.create}
          </Button>
        </div>
      </div>
    </div>
  );
}
