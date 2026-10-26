// Settings: opened from the account menu (#1496) and the sidebar. What it holds is this browser's —
// the language and the theme — and never the account's, so nothing here
// reaches the server.

import { LanguageControl } from "@/app/LanguageControl";
import { ThemeControl } from "@/app/ThemeControl";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";

/** The settings dialog, open or shut by its owner. */
export function SettingsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <SettingsContent />
    </Dialog>
  );
}

function SettingsContent() {
  const t = useT();
  return (
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{t.common.settings.title}</DialogTitle>
        <DialogDescription>{t.common.settings.description}</DialogDescription>
      </DialogHeader>
      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t.common.settings.language}</h3>
        <LanguageControl className="w-full" />
      </section>
      <section className="flex flex-col gap-2">
        <h3 className="font-medium">{t.common.settings.theme}</h3>
        <ThemeControl />
      </section>
    </DialogContent>
  );
}
