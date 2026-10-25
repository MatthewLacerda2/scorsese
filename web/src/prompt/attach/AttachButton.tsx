// The "+" in a circle that opens the file picker, hinted to what the library
// takes; what is picked goes to the tray.

import { PlusIcon } from "lucide-react";
import { useRef } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import type { Attachments } from "./useAttachments";

export function AttachButton({
  attachments,
  className,
}: {
  attachments: Attachments;
  className?: string;
}) {
  const t = useT().prompt.attach;
  const input = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button
        type="button"
        size="icon"
        variant="outline"
        aria-label={t.add}
        title={t.add}
        onClick={() => input.current?.click()}
        className={`rounded-full ${className ?? ""}`}
      >
        <PlusIcon />
      </Button>
      <input
        ref={input}
        type="file"
        multiple
        hidden
        accept={attachments.accept}
        data-testid="attach-input"
        onChange={(event) => {
          const picked = Array.from(event.target.files ?? []);
          event.target.value = "";
          attachments.add(picked);
        }}
      />
    </>
  );
}
