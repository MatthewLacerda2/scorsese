// The theme switch, in the two places it lives: a row in the account menu once
// signed in, and the corner of the login page. Either way it is three segments
// side by side — Light | Dark | System — with the user's choice marked, so
// "follow the system" (the default) is something they can see and pick back.

import { type LucideIcon, MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { type ThemeChoice, useTheme } from "@/lib/theme";
import { cn } from "@/lib/utils";

const CHOICES = [
  { choice: "light", label: "Light", Icon: SunIcon },
  { choice: "dark", label: "Dark", Icon: MoonIcon },
  { choice: "system", label: "System", Icon: MonitorIcon },
] as const satisfies readonly { choice: ThemeChoice; label: string; Icon: LucideIcon }[];

const GROUP = "flex rounded-md border p-0.5";
const SEGMENT =
  "flex flex-1 items-center justify-center gap-1 rounded-sm px-2 py-1 text-xs text-muted-foreground outline-hidden hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[active=true]:bg-muted data-[active=true]:font-medium data-[active=true]:text-foreground [&_svg]:size-3.5";

/** The control on its own, for a page with no account menu. */
export function ThemeControl({ className }: { className?: string }) {
  const { choice, choose } = useTheme();
  return (
    <fieldset className={cn(GROUP, className)}>
      <legend className="sr-only">Theme</legend>
      {CHOICES.map(({ choice: value, label, Icon }) => (
        <button
          key={value}
          type="button"
          aria-pressed={choice === value}
          data-active={choice === value}
          className={SEGMENT}
          onClick={() => choose(value)}
        >
          <Icon /> {label}
        </button>
      ))}
    </fieldset>
  );
}

/**
 * The same control as an account-menu row. Each segment is a menu item, so the
 * arrow keys reach it, and choosing one leaves the menu open to show the mark.
 */
export function ThemeMenuRow() {
  const { choice, choose } = useTheme();
  return (
    <fieldset className={cn(GROUP, "m-1")}>
      <legend className="sr-only">Theme</legend>
      {CHOICES.map(({ choice: value, label, Icon }) => (
        <DropdownMenuItem
          key={value}
          aria-checked={choice === value}
          role="menuitemradio"
          data-active={choice === value}
          className={SEGMENT}
          onSelect={(event) => {
            event.preventDefault();
            choose(value);
          }}
        >
          <Icon /> {label}
        </DropdownMenuItem>
      ))}
    </fieldset>
  );
}
