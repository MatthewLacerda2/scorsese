// Quick, Balanced or Thorough, beside Send (#769): how hard the assistant
// thinks on the next message, in plain words, with one line of hover text
// each saying what it is for (`effort.ts` has the rest).

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n/I18nProvider";
import { EFFORTS, type Effort } from "./effort";

export function EffortPicker({
  effort,
  onChange,
}: {
  effort: Effort;
  onChange: (effort: Effort) => void;
}) {
  const t = useT().chat.effort;
  return (
    <Select value={effort} onValueChange={(value) => onChange(value as Effort)}>
      <SelectTrigger size="sm" aria-label={t.label} title={t.hint[effort]}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {EFFORTS.map((level) => (
          <SelectItem key={level} value={level} title={t.hint[level]}>
            {t.name[level]}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
