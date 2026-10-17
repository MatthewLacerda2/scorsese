// "New conversation" (#1437): a button with a small confirmation just above
// it, not a modal. Start empties the chat at once; the next message opens a
// new conversation on the server, and this one stays saved there.

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";

export function NewConversation({ onStart }: { onStart: () => void }) {
  const t = useT().chat.composer;
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    const close = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [open]);
  return (
    <div className="relative">
      <Button size="sm" variant="ghost" aria-expanded={open} onClick={() => setOpen(!open)}>
        {t.fresh}
      </Button>
      {open && (
        <div
          role="dialog"
          aria-label={t.freshTitle}
          className="absolute bottom-full left-0 z-10 mb-2 flex w-64 flex-col gap-2 rounded-md border bg-popover p-3 text-sm text-popover-foreground shadow-md"
        >
          <p>{t.freshTitle}</p>
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="outline" onClick={() => setOpen(false)}>
              {t.freshCancel}
            </Button>
            <Button
              size="sm"
              onClick={() => {
                setOpen(false);
                onStart();
              }}
            >
              {t.freshStart}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
