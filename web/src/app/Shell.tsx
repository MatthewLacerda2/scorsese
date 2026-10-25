// The frame around every signed-in page: the header (where you are, what you
// have to spend, who you are) and the upload tray, which stays on screen while
// the user browses so an upload is never lost to a click, and the toasts a
// finished render raises on whichever page is open. Between the nav and
// the balance is room a page may fill with its own controls (`headerSlot.tsx`).

import { lazy, Suspense, useState } from "react";
import { NavLink, Outlet, useMatch } from "react-router";
import { AccountMenu } from "@/app/AccountMenu";
import { AddCreditsProvider } from "@/app/addCredits";
import { BalanceChip } from "@/app/BalanceChip";
import { HeaderSlotProvider } from "@/app/headerSlot";
import { PageFallback } from "@/app/PageFallback";
import { SettingsButton } from "@/app/Settings";
import { UploadTray } from "@/files/UploadTray";
import { UploadsProvider } from "@/files/uploads";
import { useT } from "@/i18n/I18nProvider";

// The toaster arrives in a chunk of its own once the shell is up, so a first
// visit (the login page, #896) never downloads Sonner. A render takes longer
// than that chunk, so no finished one is missed.
const RenderToasts = lazy(() =>
  import("@/app/RenderToasts").then((module) => ({ default: module.RenderToasts })),
);

const NAV = [
  { to: "/", key: "home" },
  { to: "/library", key: "library" },
] as const;

const LINK = ({ isActive }: { isActive: boolean }) =>
  `rounded-md px-3 py-1.5 text-sm ${isActive ? "bg-muted font-medium" : "text-muted-foreground hover:text-foreground"}`;

export function Shell() {
  // The editor takes the whole window under the header, its panels scrolling
  // on their own; every other page is a padded page that scrolls. One shell
  // either way, so the upload tray and its uploads survive moving between them.
  const edit = useMatch("/projects/:id/edit");
  const editing = edit !== null;
  const t = useT();
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return (
    <UploadsProvider>
      <AddCreditsProvider>
        <div className={editing ? "flex h-svh flex-col" : "flex min-h-svh flex-col"}>
          <header className="flex items-center gap-2 border-b px-4 py-2">
            <nav className="flex shrink-0 gap-1">
              {NAV.map(({ to, key }) => (
                <NavLink key={to} to={to} end className={LINK}>
                  {t.common.nav[key]}
                </NavLink>
              ))}
              {/* The project's files page is reached from its editor. */}
              {edit?.params.id && (
                <NavLink to={`/projects/${edit.params.id}`} end className={LINK}>
                  {t.pages.projects.files}
                </NavLink>
              )}
            </nav>
            {/* A page's own controls; empty, it still pushes the account side right. */}
            <div
              ref={setSlot}
              data-header-slot=""
              className="flex min-w-0 flex-1 items-center gap-2"
            />
            <div className="flex shrink-0 items-center gap-2">
              <BalanceChip />
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
      </AddCreditsProvider>
    </UploadsProvider>
  );
}
