// Settings: the gear in the header (#704). What it holds is this browser's —
// the language and the theme — and never the account's, so nothing here
// reaches the server.

import { SettingsIcon } from "lucide-react";
import { LanguageControl } from "@/app/LanguageControl";
import { ThemeControl } from "@/app/ThemeControl";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";

export function SettingsButton() {
  const t = useT();
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={t.common.settings.open}>
          <SettingsIcon />
        </Button>
      </DialogTrigger>
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
    </Dialog>
  );
}
