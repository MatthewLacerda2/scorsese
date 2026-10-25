// The theme switch: in the Settings panel once signed in, and on the landing
// page's header. Two segments side by side — Light | Dark — with the page's
// theme marked; light is the default. `compact` keeps only the icons, for a
// header with little room.

import { type LucideIcon, MoonIcon, SunIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { type Theme, useTheme } from "@/lib/theme";
import { cn } from "@/lib/utils";

const CHOICES = [
  { choice: "light", Icon: SunIcon },
  { choice: "dark", Icon: MoonIcon },
] as const satisfies readonly { choice: Theme; Icon: LucideIcon }[];

const GROUP = "flex rounded-md border p-0.5";
const SEGMENT =
  "h-auto flex-1 gap-1 rounded-sm px-2 py-1 text-xs text-muted-foreground outline-hidden hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[active=true]:bg-muted data-[active=true]:font-medium data-[active=true]:text-foreground [&_svg]:size-3.5";

export function ThemeControl({ className, compact }: { className?: string; compact?: boolean }) {
  const t = useT();
  const { theme, choose } = useTheme();
  return (
    <fieldset className={cn(GROUP, className)}>
      <legend className="sr-only">{t.common.settings.theme}</legend>
      {CHOICES.map(({ choice: value, Icon }) => (
        <Button
          key={value}
          type="button"
          variant="ghost"
          aria-pressed={theme === value}
          data-active={theme === value}
          className={SEGMENT}
          onClick={() => choose(value)}
        >
          <Icon />{" "}
          {compact ? (
            <span className="sr-only">{t.common.theme[value]}</span>
          ) : (
            t.common.theme[value]
          )}
        </Button>
      ))}
    </fieldset>
  );
}
