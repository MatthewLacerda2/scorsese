// The public header's language and theme on a phone (#1566): one button, a
// globe, that opens both as a short menu, so the header keeps room for the mark
// and the way in. Wider screens show the two controls themselves. The globe
// rather than a word, since a visitor who cannot read the page has to find it.

import { GlobeIcon, MoonIcon, SunIcon } from "lucide-react";
import { Flag } from "@/app/Flag";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { LANGUAGES, type Language } from "@/i18n/language";
import { type Theme, useTheme } from "@/lib/theme";
import { cn } from "@/lib/utils";

/** A row a thumb can hit: 48 px tall (#1566). */
const ROW = "min-h-12 gap-2.5 px-3 text-base";

export function PageSettings({ className }: { className?: string }) {
  const t = useT().common;
  const { language, choose: chooseLanguage } = useLanguage();
  const { theme, choose: chooseTheme } = useTheme();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-lg"
          aria-label={t.settings.open}
          className={cn(
            "relative touch-target text-paper/80 hover:bg-paper/10 hover:text-paper",
            className,
          )}
        >
          <GlobeIcon className="size-5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>{t.settings.language}</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={language}
          onValueChange={(next) => chooseLanguage(next as Language)}
        >
          {LANGUAGES.map(({ language: value, name }) => (
            <DropdownMenuRadioItem key={value} value={value} lang={value} className={ROW}>
              <Flag language={value} />
              {name}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{t.settings.theme}</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={theme} onValueChange={(next) => chooseTheme(next as Theme)}>
          <DropdownMenuRadioItem value="light" className={ROW}>
            <SunIcon />
            {t.theme.light}
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="dark" className={ROW}>
            <MoonIcon />
            {t.theme.dark}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
