// The sidebar on a phone: a menu button that slides it in from the left. The
// sidebar pages draw it in their narrow header, and the editor's phone bar
// draws the same one (#1570), so a phone has one drawer wherever it is.

import { MenuIcon } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Sidebar } from "@/home/Sidebar";
import { useT } from "@/i18n/I18nProvider";

export function SidebarSheet() {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
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
  );
}
