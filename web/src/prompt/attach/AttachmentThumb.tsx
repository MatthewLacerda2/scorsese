// One attached file in the tray: its picture (a still is drawn from the file
// itself, before any upload) or its kind's icon, its name, a ring while it is
// on its way, and an × that takes it off the prompt — never out of the library.

import {
  FileAudioIcon,
  FileCodeIcon,
  FileIcon,
  FileImageIcon,
  FileTextIcon,
  FileVideoIcon,
  XIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { ProgressRing } from "./ProgressRing";
import type { Attachment } from "./useAttachments";

const KIND_ICON = {
  video: FileVideoIcon,
  image: FileImageIcon,
  audio: FileAudioIcon,
  html: FileCodeIcon,
  text: FileTextIcon,
};

/** A still's own pixels, for as long as the thumbnail shows. */
function usePicture(file: File, still: boolean): string | undefined {
  const [url, setUrl] = useState<string>();
  useEffect(() => {
    if (!still) return;
    const made = URL.createObjectURL(file);
    setUrl(made);
    return () => URL.revokeObjectURL(made);
  }, [file, still]);
  return url;
}

export function AttachmentThumb({
  attachment,
  onRemove,
}: {
  attachment: Attachment;
  onRemove: () => void;
}) {
  const t = useT().prompt.attach;
  const { file, kind, status } = attachment;
  const picture = usePicture(file, kind === "image");
  const Icon = kind ? KIND_ICON[kind] : FileIcon;
  const failed = status.phase === "failed";
  return (
    <li
      className={`group relative w-16 shrink-0 ${failed ? "text-destructive" : ""}`}
      title={failed ? (status.message ?? t.failed) : file.name}
    >
      <div
        className={`relative flex size-16 items-center justify-center overflow-hidden rounded-xl border bg-muted ${failed ? "border-destructive" : ""}`}
      >
        {picture ? (
          <img src={picture} alt="" className="absolute inset-0 size-full object-cover" />
        ) : (
          <Icon className="size-6 text-muted-foreground" aria-hidden />
        )}
        {status.phase === "uploading" && (
          <div className="absolute inset-0 flex items-center justify-center bg-background/60">
            <ProgressRing progress={status.progress} />
          </div>
        )}
      </div>
      <p className="mt-1 truncate text-xs">{file.name}</p>
      <Button
        type="button"
        size="icon-xs"
        variant="secondary"
        aria-label={t.remove(file.name)}
        onClick={onRemove}
        className="absolute -top-1.5 -right-1.5 rounded-full border shadow-sm"
      >
        <XIcon />
      </Button>
    </li>
  );
}
