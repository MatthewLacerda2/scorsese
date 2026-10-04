// The frame around every signed-in page: the header (where you are, what you
// have to spend, who you are) and the upload tray, which stays on screen while
// the user browses so an upload is never lost to a click.

import { Link, NavLink, Outlet, useMatch, useNavigate } from "react-router";
import { useBalance } from "@/app/queries";
import { SettingsButton } from "@/app/Settings";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { UploadTray } from "@/files/UploadTray";
import { UploadsProvider } from "@/files/uploads";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { useAccount, useLogout } from "@/session/session";

const NAV = [
  { to: "/projects", key: "projects" },
  { to: "/library", key: "library" },
] as const;

export function Shell() {
  // The editor takes the whole window under the header, its panels scrolling
  // on their own; every other page is a padded page that scrolls. One shell
  // either way, so the upload tray and its uploads survive moving between them.
  const editing = useMatch("/projects/:id/edit") !== null;
  const t = useT();
  return (
    <UploadsProvider>
      <div className={editing ? "flex h-svh flex-col" : "flex min-h-svh flex-col"}>
        <header className="flex items-center gap-2 border-b px-4 py-2">
          <Link
            to="/projects"
            className="mr-4 rounded-md border px-2.5 py-0.5 font-heading text-lg font-semibold hover:bg-muted"
          >
            scorsese
          </Link>
          <nav className="flex gap-1">
            {NAV.map(({ to, key }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) =>
                  `rounded-md px-3 py-1.5 text-sm ${isActive ? "bg-muted font-medium" : "text-muted-foreground hover:text-foreground"}`
                }
              >
                {t.common.nav[key]}
              </NavLink>
            ))}
          </nav>
          <div className="ml-auto flex items-center gap-2">
            <BalanceChip />
            <SettingsButton />
            <AccountMenu />
          </div>
        </header>
        <main className={editing ? "min-h-0 flex-1" : "flex-1 p-4 md:p-6"}>
          <Outlet />
        </main>
        <UploadTray />
      </div>
    </UploadsProvider>
  );
}

/** The balance, in dollars; a click opens the history. */
function BalanceChip() {
  const balance = useBalance();
  const t = useT();
  if (!balance.data) return null;
  const { balance_micros } = balance.data;
  return (
    <Button asChild variant="outline" size="sm" title={t.common.balanceTitle}>
      <Link to="/spending">
        <span className={balance_micros < 0 ? "text-destructive" : undefined}>
          {formatDollars(balance_micros)}
        </span>
      </Link>
    </Button>
  );
}

function AccountMenu() {
  const account = useAccount();
  const logout = useLogout();
  const navigate = useNavigate();
  const t = useT();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="rounded-full" aria-label={t.common.account}>
          <img src="/icon.png" alt="" className="size-7 rounded-full" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel className="font-normal text-muted-foreground">
          {account.data?.email}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => navigate("/spending")}>
          {t.common.spendingHistory}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={() => logout.mutate(undefined, { onSettled: () => navigate("/login") })}
        >
          {t.common.logOut}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
