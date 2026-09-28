// The frame around every signed-in page: the header (where you are, what you
// have to spend, who you are) and the upload tray, which stays on screen while
// the user browses so an upload is never lost to a click.

import { CircleUserIcon } from "lucide-react";
import { Link, NavLink, Outlet, useNavigate } from "react-router";
import { useBalance } from "@/app/queries";
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
import { formatDollars, formatMoney } from "@/lib/money";
import { useAccount, useLogout } from "@/session/session";

const NAV = [
  { to: "/projects", label: "Projects" },
  { to: "/library", label: "Library" },
];

export function Shell() {
  return (
    <UploadsProvider>
      <div className="flex min-h-svh flex-col">
        <header className="flex items-center gap-2 border-b px-4 py-2">
          <Link to="/projects" className="mr-4 font-heading text-lg font-semibold">
            scorsese
          </Link>
          <nav className="flex gap-1">
            {NAV.map(({ to, label }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) =>
                  `rounded-md px-3 py-1.5 text-sm ${isActive ? "bg-muted font-medium" : "text-muted-foreground hover:text-foreground"}`
                }
              >
                {label}
              </NavLink>
            ))}
          </nav>
          <div className="ml-auto flex items-center gap-2">
            <BalanceChip />
            <AccountMenu />
          </div>
        </header>
        <main className="flex-1 p-4 md:p-6">
          <Outlet />
        </main>
        <UploadTray />
      </div>
    </UploadsProvider>
  );
}

/** The balance, ≈ reais with dollars beside; a click opens the history. */
function BalanceChip() {
  const balance = useBalance();
  if (!balance.data) return null;
  const { balance_micros, balance_centavos } = balance.data;
  return (
    <Button asChild variant="outline" size="sm" title="Your credits — see what you spent">
      <Link to="/spending">
        <span className={balance_micros < 0 ? "text-destructive" : undefined}>
          {formatMoney(balance_micros, balance_centavos)}
        </span>
        {balance_centavos !== null && (
          <span className="text-muted-foreground">{formatDollars(balance_micros)}</span>
        )}
      </Link>
    </Button>
  );
}

function AccountMenu() {
  const account = useAccount();
  const logout = useLogout();
  const navigate = useNavigate();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="Your account">
          <CircleUserIcon />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel className="font-normal text-muted-foreground">
          {account.data?.email}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => navigate("/spending")}>Spending history</DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => logout.mutate(undefined, { onSettled: () => navigate("/login") })}
        >
          Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
