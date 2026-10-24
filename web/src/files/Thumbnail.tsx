// A library file's picture, for a tile. It is drawn by a job after the file
// arrives and answers 404 until then, so a missing one shows the kind's icon
// and is asked for again a little later.

import {
  FileAudioIcon,
  FileCodeIcon,
  FileImageIcon,
  FileTextIcon,
  FileVideoIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { FileKind } from "@/api";

const KIND_ICON = {
  video: FileVideoIcon,
  image: FileImageIcon,
  audio: FileAudioIcon,
  html: FileCodeIcon,
  text: FileTextIcon,
};

/** How often, and how many times, a thumbnail still being drawn is re-asked. */
const RETRY_MS = 4000;
const RETRIES = 8;

/** Kinds the server never draws a picture of: it answers `404 {"pending": false}` at once. */
const NO_PICTURE: readonly FileKind[] = ["html", "text"];

export function Thumbnail({ src, kind }: { src: string; kind: FileKind }) {
  const [attempt, setAttempt] = useState(0);
  const [failed, setFailed] = useState(false);
  const never = NO_PICTURE.includes(kind);
  const missing = failed || never;
  useEffect(() => {
    if (!failed || never || attempt >= RETRIES) return;
    const timer = setTimeout(() => {
      setFailed(false);
      setAttempt((n) => n + 1);
    }, RETRY_MS);
    return () => clearTimeout(timer);
  }, [failed, never, attempt]);
  const Icon = KIND_ICON[kind];
  return (
    <div className="relative flex aspect-video items-center justify-center overflow-hidden bg-muted">
      {missing ? (
        <Icon className="size-10 text-muted-foreground" aria-hidden />
      ) : (
        <img
          // A changed URL is what makes the browser ask again after a 404.
          src={attempt === 0 ? src : `${src}?attempt=${attempt}`}
          alt=""
          loading="lazy"
          className="absolute inset-0 size-full object-cover"
          onError={() => setFailed(true)}
        />
      )}
    </div>
  );
}
