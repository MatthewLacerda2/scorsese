// The frame of the signed-in pages that have the sidebar: the home, the
// library and the spending history. The sidebar sits fixed beside the page on a
// wide screen, and in a sheet behind a menu button on a narrow one. What the
// shell gives its pages is provided here too (the upload tray and its uploads,
// the Add credits dialog, the toasts a finished render raises), so an upload
// survives moving between these pages. The page is `children`, or what the
// route nests (`Outlet`) in a padded, scrolling area.

import { MenuIcon } from "lucide-react";
import { lazy, type ReactNode, Suspense, useState } from "react";
import { Outlet } from "react-router";
import { AddCreditsProvider } from "@/app/addCredits";
import { PageFallback } from "@/app/PageFallback";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { UploadTray } from "@/files/UploadTray";
import { UploadsProvider } from "@/files/uploads";
import { Sidebar } from "@/home/Sidebar";
import { useT } from "@/i18n/I18nProvider";

const RenderToasts = lazy(() =>
  import("@/app/RenderToasts").then((module) => ({ default: module.RenderToasts })),
);

export function SidebarLayout({ children }: { children?: ReactNode }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <UploadsProvider>
      <AddCreditsProvider>
        <div className="flex h-svh bg-background text-foreground">
          <aside className="hidden w-64 shrink-0 border-r bg-muted/30 md:block">
            <Sidebar />
          </aside>
          <div className="flex min-w-0 flex-1 flex-col">
            <header className="flex items-center gap-2 p-2 md:hidden">
              <Sheet open={open} onOpenChange={setOpen}>
                <SheetTrigger asChild>
                  <Button variant="ghost" size="icon" aria-label={t.home.menu}>
                    <MenuIcon />
                  </Button>
                </SheetTrigger>
                <SheetContent side="left" showCloseButton={false} className="gap-0 p-0">
                  <SheetTitle className="sr-only">{t.home.nav}</SheetTitle>
                  <Sidebar onNavigate={() => setOpen(false)} />
                </SheetContent>
              </Sheet>
            </header>
            <main className="flex min-h-0 flex-1 flex-col overflow-y-auto">
              {children ?? (
                <div className="p-4 md:p-6">
                  <Suspense fallback={<PageFallback />}>
                    <Outlet />
                  </Suspense>
                </div>
              )}
            </main>
          </div>
        </div>
        <UploadTray />
        <Suspense fallback={null}>
          <RenderToasts />
        </Suspense>
      </AddCreditsProvider>
    </UploadsProvider>
  );
}
