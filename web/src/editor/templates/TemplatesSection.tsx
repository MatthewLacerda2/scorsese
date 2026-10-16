// The user's templates, in the editor's sidebar (#546, #702): each goes in at the
// playhead with one `template_insert` — where its tracks land is `core`'s rule
// (docs/web.md, *Templates*), and the answer carries the project as it is now.
// A template is let go of here too, which is also what frees a library file
// only a template still holds.

import { useQuery } from "@tanstack/react-query";
import { PlusIcon, Trash2Icon } from "lucide-react";
import { api, type Fps, type TemplateSummary } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useInvalidatingMutation } from "@/app/useInvalidatingMutation";
import { Button } from "@/components/ui/button";
import { QueryError } from "@/components/ui/query-error";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { formatNumber } from "@/lib/format";
import type { EditOutcome } from "../project";
import { toSeconds } from "../timeline/time";

function useTemplates() {
  return useQuery({ queryKey: queryKeys.templates, queryFn: api.templates.list });
}

interface Props {
  edit: EditOutcome;
  playhead: number;
  fps: Fps;
}

export function TemplatesSection({ edit, playhead, fps }: Props) {
  const t = useT().editor.templates;
  const { language } = useLanguage();
  // Seconds to one decimal, in the reader's own way of writing a decimal.
  const tenths = (seconds: number) => formatNumber(seconds, { digits: 1 }, language);
  const templates = useTemplates();
  const remove = useInvalidatingMutation([queryKeys.templates], {
    mutationFn: (id: number) => api.templates.remove(id),
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
      <QueryError error={templates.error} className="text-xs text-destructive" />
      {templates.data?.length === 0 && <p className="text-xs text-muted-foreground">{t.none}</p>}
      <QueryError error={remove.error} className="text-xs text-destructive" />
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
              {t.summary(tenths(template.seconds), template.clips)}
            </p>
          </div>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t.insert(template.name)}
            title={t.insertTitle(tenths(at))}
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
