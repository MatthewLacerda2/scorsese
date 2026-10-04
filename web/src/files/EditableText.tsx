// Text that becomes a field on click and is saved on Enter or leaving it —
// the way a file is renamed in Drive. Escape puts the old text back.

import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";

interface Props {
  label: string;
  value: string;
  onSave: (value: string) => Promise<unknown>;
  multiline?: boolean;
  placeholder?: string;
  className?: string;
}

export function EditableText({ label, value, onSave, multiline, placeholder, className }: Props) {
  const t = useT();
  const [draft, setDraft] = useState(value);
  const [editing, setEditing] = useState(false);
  // Set by Escape, so a blur that follows it does not save what was abandoned.
  const cancelled = useRef(false);
  useEffect(() => setDraft(value), [value]);

  const save = async () => {
    setEditing(false);
    if (cancelled.current || draft.trim() === value.trim()) return;
    try {
      await onSave(draft);
    } catch {
      // The caller shows the refusal; the field goes back to what is stored.
      setDraft(value);
    }
  };

  if (!editing) {
    return (
      <button
        type="button"
        aria-label={t.files.edit(label)}
        onClick={() => {
          cancelled.current = false;
          setEditing(true);
        }}
        className={`rounded-md px-1 -mx-1 text-left whitespace-pre-wrap hover:bg-muted ${value ? "" : "text-muted-foreground"} ${className ?? ""}`}
      >
        {value || placeholder || "—"}
      </button>
    );
  }

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Enter" && !multiline) event.currentTarget.blur();
    if (event.key === "Escape") {
      // Only the field lets go of Escape; the panel around it stays open.
      event.stopPropagation();
      cancelled.current = true;
      setDraft(value);
      setEditing(false);
    }
  };
  const common = {
    "aria-label": label,
    autoFocus: true,
    value: draft,
    placeholder,
    onBlur: save,
    onKeyDown,
  };
  return multiline ? (
    <Textarea {...common} onChange={(event) => setDraft(event.target.value)} />
  ) : (
    <Input {...common} className={className} onChange={(event) => setDraft(event.target.value)} />
  );
}
