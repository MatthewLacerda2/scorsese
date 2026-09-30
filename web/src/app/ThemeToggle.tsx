// The light/dark switch, in the two places it lives: an item in the account
// menu once signed in, and a button in the corner of the login page. Both say
// what a click will do ("Dark mode" while the page is light), not what it is.

import { MoonIcon, SunIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { useTheme } from "@/lib/theme";

/** An icon button, for a page with no account menu. */
export function ThemeToggle({ className }: { className?: string }) {
  const { theme, toggle } = useTheme();
  const label = theme === "dark" ? "Light mode" : "Dark mode";
  return (
    <Button
      variant="ghost"
      size="icon"
      className={className}
      onClick={toggle}
      aria-label={label}
      title={label}
    >
      {theme === "dark" ? <SunIcon /> : <MoonIcon />}
    </Button>
  );
}

/** The same switch as an account-menu item. */
export function ThemeMenuItem() {
  const { theme, toggle } = useTheme();
  return (
    <DropdownMenuItem onSelect={toggle}>
      {theme === "dark" ? <SunIcon /> : <MoonIcon />}
      {theme === "dark" ? "Light mode" : "Dark mode"}
    </DropdownMenuItem>
  );
}
