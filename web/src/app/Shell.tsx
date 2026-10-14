// The frame around every signed-in page: the header (where you are, what you
// have to spend, who you are) and the upload tray, which stays on screen while
// the user browses so an upload is never lost to a click, and the toasts a
// finished render raises on whichever page is open. Between the nav and
// the balance is room a page may fill with its own controls (`headerSlot.tsx`).

import { PlusIcon } from "lucide-react";
import { lazy, Suspense, useState } from "react";
import { Link, NavLink, Outlet, useMatch } from "react-router";
import { AccountMenu } from "@/app/AccountMenu";
import { HeaderSlotProvider } from "@/app/headerSlot";
import { PageFallback } from "@/app/PageFallback";
import { useBalance } from "@/app/queries";
import { SettingsButton } from "@/app/Settings";
import { Button } from "@/components/ui/button";
import { UploadTray } from "@/files/UploadTray";
import { UploadsProvider } from "@/files/uploads";
import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";

// The toaster arrives in a chunk of its own once the shell is up, so a first
// visit (the login page, #896) never downloads Sonner. A render takes longer
// than that chunk, so no finished one is missed.
const RenderToasts = lazy(() =>
  import("@/app/RenderToasts").then((module) => ({ default: module.RenderToasts })),
);

// Buying credits (#548) is a dialog of its own chunk, fetched on the first
// click of the button beside the balance.
const AddCreditsDialog = lazy(() =>
  import("@/payments/AddCredits").then((module) => ({ default: module.AddCreditsDialog })),
);

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
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return (
    <UploadsProvider>
      <div className={editing ? "flex h-svh flex-col" : "flex min-h-svh flex-col"}>
        <header className="flex items-center gap-2 border-b px-4 py-2">
          <nav className="flex shrink-0 gap-1">
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
          {/* A page's own controls; empty, it still pushes the account side right. */}
          <div
            ref={setSlot}
            data-header-slot=""
            className="flex min-w-0 flex-1 items-center gap-2"
          />
          <div className="flex shrink-0 items-center gap-2">
            <BalanceChip />
            <AddCredits />
            <SettingsButton />
            <AccountMenu />
          </div>
        </header>
        <main className={editing ? "min-h-0 flex-1" : "flex-1 p-4 md:p-6"}>
          <HeaderSlotProvider value={slot}>
            <Suspense fallback={<PageFallback />}>
              <Outlet />
            </Suspense>
          </HeaderSlotProvider>
        </main>
        <UploadTray />
        <Suspense fallback={null}>
          <RenderToasts />
        </Suspense>
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

/** The button that opens "Add credits", and the dialog once it has been opened. */
function AddCredits() {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [opened, setOpened] = useState(false);
  return (
    <>
      <Button
        variant="outline"
        size="icon-sm"
        aria-label={t.buying.add}
        title={t.buying.add}
        onClick={() => {
          setOpened(true);
          setOpen(true);
        }}
      >
        <PlusIcon />
      </Button>
      {opened && (
        <Suspense fallback={null}>
          <AddCreditsDialog open={open} onOpenChange={setOpen} />
        </Suspense>
      )}
    </>
  );
}
