// The language picker, in the Settings panel and the corner of the login page
// (a reader who cannot read the page yet still has to find it). Each language
// is named in itself, beside its flag (#770), so the list reads the same
// whatever is chosen.

import { Flag } from "@/app/Flag";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { LANGUAGES, type Language } from "@/i18n/language";

export function LanguageControl({ className }: { className?: string }) {
  const t = useT();
  const { language, choose } = useLanguage();
  const chosen = LANGUAGES.find((entry) => entry.language === language);
  return (
    <Select value={language} onValueChange={(next) => choose(next as Language)}>
      <SelectTrigger size="sm" aria-label={t.common.settings.language} className={className}>
        {/* Drawn here rather than copied from the open list, so the closed
            picker shows the flag before the list has ever been opened. */}
        <SelectValue>
          <Flag language={language} />
          {chosen?.name}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        {LANGUAGES.map(({ language: value, name }) => (
          <SelectItem key={value} value={value} lang={value}>
            <Flag language={value} />
            {name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
