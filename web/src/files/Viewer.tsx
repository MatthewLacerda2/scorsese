// Opening a file: the picture, or the video or sound playing. The browser
// streams it from `GET /api/library/{id}/file` in ranges, so a long video
// starts at once and seeks without being fetched whole.

import { useEffect, useState } from "react";
import { api, type FileKind } from "@/api";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { useT } from "@/i18n/I18nProvider";

export interface Opened {
  id: number;
  name: string;
  kind: FileKind;
}

export function Viewer({ file, onClose }: { file: Opened | null; onClose: () => void }) {
  return (
    <Dialog open={file !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-4xl">
        {file && (
          <>
            <DialogTitle className="truncate pr-8">{file.name}</DialogTitle>
            <Media file={file} />
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function Media({ file }: { file: Opened }) {
  const src = api.library.fileUrl(file.id);
  switch (file.kind) {
    case "image":
      return <img src={src} alt={file.name} className="max-h-[75vh] w-full object-contain" />;
    case "video":
      // biome-ignore lint/a11y/useMediaCaption: a user's own footage has no caption track to offer.
      return <video src={src} controls autoPlay className="max-h-[75vh] w-full bg-black" />;
    case "audio":
      // biome-ignore lint/a11y/useMediaCaption: a user's own recording has no caption track to offer.
      return <audio src={src} controls autoPlay className="w-full" />;
    case "html":
      return <PageSource id={file.id} />;
  }
}

/** A page is shown as its source, read-only: it is served as `text/plain` and never run here. */
function PageSource({ id }: { id: number }) {
  const t = useT();
  const [text, setText] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let live = true;
    api.library
      .fileText(id)
      .then((body) => live && setText(body))
      .catch(() => live && setFailed(true));
    return () => {
      live = false;
    };
  }, [id]);
  if (failed) return <p className="text-sm text-muted-foreground">{t.files.viewer.failed}</p>;
  if (text === null)
    return <p className="text-sm text-muted-foreground">{t.files.viewer.loading}</p>;
  return (
    <pre className="max-h-[75vh] overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap">
      {text}
    </pre>
  );
}
