// A number the user types, sent when they are done with it: on Enter or on
// leaving the field, and only if it changed. Each value is a round trip to the
// server, so a keystroke is not an edit. Escape puts back what the document says.

import { useEffect, useState } from "react";

interface Props {
  value: number;
  prefix?: string;
  suffix?: string;
  min?: number;
  onCommit: (value: number) => void;
}

/** A value as a field shows it: two decimals at most, no trailing zeros. */
export function tidy(value: number): string {
  return String(Math.round(value * 100) / 100);
}

export function NumberField({ value, prefix, suffix, min, onCommit }: Props) {
  const [draft, setDraft] = useState(tidy(value));
  // The document is the model: when it changes — an edit landed, or was
  // refused and nothing moved — the field shows what it says now.
  useEffect(() => setDraft(tidy(value)), [value]);
  const commit = () => {
    const typed = Number(draft);
    if (draft.trim() === "" || !Number.isFinite(typed) || (min !== undefined && typed < min)) {
      setDraft(tidy(value));
      return;
    }
    if (tidy(typed) !== tidy(value)) onCommit(typed);
  };
  return (
    <label className="flex h-8 min-w-0 flex-1 items-center gap-1 rounded-md border px-2 focus-within:ring-2 focus-within:ring-ring/50">
      {prefix && <span className="text-xs text-muted-foreground">{prefix}</span>}
      <input
        inputMode="decimal"
        className="w-full min-w-0 bg-transparent outline-none"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") setDraft(tidy(value));
        }}
      />
      {suffix && <span className="text-xs text-muted-foreground">{suffix}</span>}
    </label>
  );
}
