// The sidebar's "Settings" row: it opens the same dialog as the account menu.

import { SettingsIcon } from "lucide-react";
import { useState } from "react";
import { SettingsDialog } from "@/app/Settings";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

export function SettingsButton() {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="ghost" className="w-full justify-start" onClick={() => setOpen(true)}>
        <SettingsIcon /> {t.common.settings.open}
      </Button>
      <SettingsDialog open={open} onOpenChange={setOpen} />
    </>
  );
}
