// The files attached to a prompt, as thumbnails above the text — the tray a
// chatbot shows — and, under them, the files a pick or a drop refused, by name.

import { useT } from "@/i18n/I18nProvider";
import { AttachmentThumb } from "./AttachmentThumb";
import type { Attachments } from "./useAttachments";

export function AttachmentTray({
  attachments,
  className = "px-4 pt-3",
}: {
  attachments: Attachments;
  /** Its padding inside the box it sits in. */
  className?: string;
}) {
  const t = useT().prompt.attach;
  const { list, refused, remove } = attachments;
  if (list.length === 0 && refused.length === 0) return null;
  return (
    <div className={`flex flex-col gap-1 ${className}`}>
      {list.length > 0 && (
        <ul aria-label={t.tray} className="flex gap-3 overflow-x-auto pt-1.5 pr-1.5 pb-1">
          {list.map((attachment) => (
            <AttachmentThumb
              key={attachment.key}
              attachment={attachment}
              onRemove={() => remove(attachment.key)}
            />
          ))}
        </ul>
      )}
      {refused.map((name) => (
        <p key={name} role="alert" className="text-xs text-destructive">
          {t.refused(name)}
        </p>
      ))}
    </div>
  );
}
