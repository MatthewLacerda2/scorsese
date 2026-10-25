// What an element shows while files are dragged over it: drop them here.

import { PaperclipIcon } from "lucide-react";
import { useT } from "@/i18n/I18nProvider";

export function DropOverlay({ rounded }: { rounded?: string }) {
  const t = useT().prompt.attach;
  return (
    <div
      className={`pointer-events-none absolute inset-0 z-50 flex items-center justify-center border-2 border-dashed border-primary bg-background/80 ${rounded ?? ""}`}
    >
      <p className="flex items-center gap-2 text-sm font-medium">
        <PaperclipIcon className="size-4" aria-hidden />
        {t.drop}
      </p>
    </div>
  );
}
