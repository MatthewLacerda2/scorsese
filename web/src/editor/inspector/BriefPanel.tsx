// A generated clip's brief in the inspector: read-only, because the web
// editor's direct edits (`http::editor::EDITS`) do not include editing a
// brief (`asset_set`'s `prompt`) — a changed brief is a sentence to the
// assistant, which also owns the quote before anything is spent (#538).

import { useT } from "@/i18n/I18nProvider";
import type { Brief } from "./brief";

export function BriefPanel({ brief }: { brief: Brief }) {
  const t = useT();
  const meaning = t.inspector.brief.states[brief.state];
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t.inspector.brief.title}
      </h3>
      <p className="text-xs" data-state={brief.state}>
        <span className="font-medium">{t.editor.states[brief.state] ?? brief.state}</span>
        {meaning && <span className="text-muted-foreground"> · {meaning}</span>}
      </p>
      <div className="grid grid-cols-[5.5rem_1fr] gap-x-2 gap-y-1 text-xs">
        <span>{brief.label}</span>
        <span className="whitespace-pre-wrap break-words">
          {brief.text ?? <em className="text-muted-foreground">{t.inspector.brief.noneYet}</em>}
        </span>
        {brief.choices.map(([name, value]) => (
          <span key={name} className="contents">
            <span>{name}</span>
            <span>{value}</span>
          </span>
        ))}
      </div>
      <p className="text-xs text-muted-foreground">{t.inspector.brief.note}</p>
    </div>
  );
}
