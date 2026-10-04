// The user's templates, in the editor's sidebar (#546, #702): each goes in at the
// playhead with one `template_insert` — where its tracks land is `core`'s rule
// (docs/web.md, *Templates*), and the answer carries the project as it is now.
// A template is let go of here too, which is also what frees a library file
// only a template still holds.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PlusIcon, Trash2Icon } from "lucide-react";
import { api, type Fps, type TemplateSummary } from "@/api";
import { Button } from "@/components/ui/button";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import type { EditOutcome } from "../project";
import { toSeconds } from "../timeline/time";

export function useTemplates() {
  return useQuery({ queryKey: ["templates"], queryFn: api.templates.list });
}

interface Props {
  edit: EditOutcome;
  playhead: number;
  fps: Fps;
}

export function TemplatesSection({ edit, playhead, fps }: Props) {
  const queryClient = useQueryClient();
  const t = useT().editor.templates;
  const { language } = useLanguage();
  // Seconds to one decimal, in the reader's own way of writing a decimal.
  const tenths = new Intl.NumberFormat(language, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
  const templates = useTemplates();
  const remove = useMutation({
    mutationFn: (id: number) => api.templates.remove(id),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["templates"] }),
  });
  const at = toSeconds(playhead, fps);
  const insert = (template: TemplateSummary) =>
    void edit.run({
      tool: "template_insert",
      args: { template: template.id, at_seconds: at },
      edit: false,
    });

  return (
    <section className="flex flex-col gap-1">
      <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t.title}
      </h2>
      {templates.isError && <p className="text-xs text-destructive">{templates.error.message}</p>}
      {templates.data?.length === 0 && <p className="text-xs text-muted-foreground">{t.none}</p>}
      {remove.isError && <p className="text-xs text-destructive">{remove.error.message}</p>}
      {templates.data?.map((template) => (
        <div key={template.id} className="flex items-center gap-1 rounded-md border px-2 py-1">
          <div
            className="min-w-0 flex-1"
            title={template.description ?? template.assets.join(", ")}
          >
            <p className="truncate text-sm">{template.name}</p>
            {template.description && (
              <p className="truncate text-xs text-muted-foreground">{template.description}</p>
            )}
            <p className="text-xs text-muted-foreground">
              {t.summary(tenths.format(template.seconds), template.clips)}
            </p>
          </div>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t.insert(template.name)}
            title={t.insertTitle(tenths.format(at))}
            disabled={edit.pending}
            onClick={() => insert(template)}
          >
            <PlusIcon />
          </Button>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t.remove(template.name)}
            title={t.removeTitle}
            disabled={remove.isPending}
            onClick={() => {
              if (window.confirm(t.confirmRemove(template.name))) {
                remove.mutate(template.id);
              }
            }}
          >
            <Trash2Icon />
          </Button>
        </div>
      ))}
    </section>
  );
}
