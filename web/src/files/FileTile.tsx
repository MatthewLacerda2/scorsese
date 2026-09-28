// One file in the grid: thumbnail, name, kind and size — nothing else, since
// a list carries only those (#527). The thumbnail is drawn by a job after the
// file arrives and answers 404 until then, so a missing one shows the kind's
// icon and is asked for again a little later.

import { FileAudioIcon, FileImageIcon, FileVideoIcon } from "lucide-react";
import { useEffect, useState } from "react";
import type { FileKind, LibraryTile } from "@/api";
import { formatBytes } from "@/lib/format";

export const KIND_ICON = { video: FileVideoIcon, image: FileImageIcon, audio: FileAudioIcon };

/** How often, and how many times, a thumbnail still being drawn is re-asked. */
const RETRY_MS = 4000;
const RETRIES = 8;

interface Props {
  tile: LibraryTile;
  selected: boolean;
  onSelect: () => void;
  onOpen: () => void;
}

export function FileTile({ tile, selected, onSelect, onOpen }: Props) {
  return (
    <button
      type="button"
      onClick={onSelect}
      onDoubleClick={onOpen}
      className={`flex flex-col overflow-hidden rounded-lg border text-left transition-colors hover:bg-muted/50 ${selected ? "border-primary ring-2 ring-primary/30" : ""}`}
    >
      <Thumbnail src={tile.thumbnail} kind={tile.kind} />
      <span className="truncate px-2 pt-2 text-sm font-medium" title={tile.name}>
        {tile.name}
      </span>
      <span className="px-2 pb-2 text-xs text-muted-foreground capitalize">
        {tile.kind} · {formatBytes(tile.size_bytes)}
      </span>
    </button>
  );
}

export function Thumbnail({ src, kind }: { src: string; kind: FileKind }) {
  const [attempt, setAttempt] = useState(0);
  const [missing, setMissing] = useState(false);
  useEffect(() => {
    if (!missing || attempt >= RETRIES) return;
    const timer = setTimeout(() => {
      setMissing(false);
      setAttempt((n) => n + 1);
    }, RETRY_MS);
    return () => clearTimeout(timer);
  }, [missing, attempt]);
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
          onError={() => setMissing(true)}
        />
      )}
    </div>
  );
}
