// A generated clip's brief in the inspector: read-only, because the web
// editor's direct edits (`http::editor::EDITS`) do not include `rebrief` — a
// changed brief is a sentence to the assistant, which also owns the quote
// before anything is spent (#538).

import { type Brief, STATES } from "./brief";

export function BriefPanel({ brief }: { brief: Brief }) {
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Brief</h3>
      <p className="text-xs" data-state={brief.state}>
        <span className="font-medium">{brief.state}</span>
        {STATES[brief.state] && (
          <span className="text-muted-foreground"> · {STATES[brief.state]}</span>
        )}
      </p>
      <div className="grid grid-cols-[5.5rem_1fr] gap-x-2 gap-y-1 text-xs">
        <span>{brief.label}</span>
        <span className="whitespace-pre-wrap break-words">
          {brief.text ?? <em className="text-muted-foreground">none yet</em>}
        </span>
        {brief.choices.map(([name, value]) => (
          <span key={name} className="contents">
            <span>{name}</span>
            <span>{value}</span>
          </span>
        ))}
      </div>
      <p className="text-xs text-muted-foreground">
        Changing the brief, or making it, is a sentence to the assistant.
      </p>
    </div>
  );
}
