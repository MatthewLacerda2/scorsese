// The new-project dialog's four steps (#1016); `NewProjectButton.tsx` says
// what each is for and why Create works from any of them.

import { cn } from "cn";
import { PlusIcon } from "lucide-react";
import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router";
import { api } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useInvalidatingMutation } from "@/app/useInvalidatingMutation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { QueryError } from "@/components/ui/query-error";
import { saveShape } from "@/editor/shape";
import { FileBrowser } from "@/files/FileBrowser";
import { useT } from "@/i18n/I18nProvider";
import { shapeOf } from "@/start/menu";
import { PlatformPicker } from "@/start/PlatformPicker";
import { StylePicker } from "@/start/StylePicker";
import { useStyleMenu } from "@/start/useStyleMenu";
import { bodyOf, type Draft, EMPTY, onPlatform, STEPS, toggled } from "./draft";

/** The steps — mounted only while the modal is open, so a new one starts blank. */
export function Wizard({ initial = EMPTY }: { initial?: Draft }) {
  const t = useT().start;
  const navigate = useNavigate();
  const menu = useStyleMenu();
  const [step, setStep] = useState(0);
  const [draft, setDraft] = useState(initial);
  const [missing, setMissing] = useState(false);
  const create = useInvalidatingMutation([queryKeys.projects, queryKeys.library], {
    mutationFn: api.projects.create,
    onSuccess: (project) => {
      const shape = menu.data ? shapeOf(menu.data, draft.platform) : null;
      if (shape) saveShape(project.id, shape);
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
      <QueryError error={create.error} />
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
