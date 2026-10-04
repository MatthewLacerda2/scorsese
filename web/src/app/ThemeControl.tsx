// The theme switch, in the two places it lives: the Settings panel once signed
// in, and the corner of the login page. Either way it is three segments side
// by side — Light | Dark | System — with the user's choice marked, so "follow
// the system" (the default) is something they can see and pick back.

import { type LucideIcon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useT } from "@/i18n/I18nProvider";
import { type ThemeChoice, useTheme } from "@/lib/theme";
import { cn } from "@/lib/utils";

const CHOICES = [
  { choice: "light", Icon: SunIcon },
  { choice: "dark", Icon: MoonIcon },
  { choice: "system", Icon: MonitorIcon },
] as const satisfies readonly { choice: ThemeChoice; Icon: LucideIcon }[];

const GROUP = "flex rounded-md border p-0.5";
const SEGMENT =
  "flex flex-1 items-center justify-center gap-1 rounded-sm px-2 py-1 text-xs text-muted-foreground outline-hidden hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[active=true]:bg-muted data-[active=true]:font-medium data-[active=true]:text-foreground [&_svg]:size-3.5";

export function ThemeControl({ className }: { className?: string }) {
  const t = useT();
  const { choice, choose } = useTheme();
  return (
    <fieldset className={cn(GROUP, className)}>
      <legend className="sr-only">{t.common.settings.theme}</legend>
      {CHOICES.map(({ choice: value, Icon }) => (
        <button
          key={value}
          type="button"
          aria-pressed={choice === value}
          data-active={choice === value}
          className={SEGMENT}
          onClick={() => choose(value)}
        >
          <Icon /> {t.common.theme[value]}
        </button>
      ))}
    </fieldset>
  );
}
