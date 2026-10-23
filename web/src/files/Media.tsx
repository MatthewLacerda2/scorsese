// The file itself — the picture, or the video or sound playing, or a page's
// source or a document's text — shared by the library's viewer and the editor's asset panel.

import { useEffect, useState } from "react";
import { api } from "@/api";
import { useT } from "@/i18n/I18nProvider";
import type { Opened } from "./Viewer";

export function Media({ file }: { file: Opened }) {
  const src = api.library.fileUrl(file.id);
  switch (file.kind) {
    case "image":
      return <img src={src} alt={file.name} className="max-h-[75vh] w-full object-contain" />;
    case "video":
      // biome-ignore lint/a11y/useMediaCaption: a user's own footage has no caption track to offer.
      return <video src={src} controls autoPlay className="max-h-[75vh] w-full bg-screen" />;
    case "audio":
      // biome-ignore lint/a11y/useMediaCaption: a user's own recording has no caption track to offer.
      return <audio src={src} controls autoPlay className="w-full" />;
    case "html":
    case "text":
      return <TextSource id={file.id} />;
  }
}

/**
 * A page is shown as its source and a document as written, read-only: both are
 * served as `text/plain`, so a page is never run here.
 */
function TextSource({ id }: { id: number }) {
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
