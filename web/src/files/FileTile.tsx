// One file in the grid: thumbnail, name, kind and size — nothing else, since
// a list carries only those (#527). A tile can be `disabled` with a `note`
// saying why (the editor's picking modal, #702), and carries `actions` beside
// its main button — buttons of their own, since a button cannot hold one. The thumbnail is drawn by a job after the
// file arrives and answers 404 until then, so a missing one shows the kind's
// icon and is asked for again a little later. A MIDI file never has one — it
// is notes, not a picture or a sound — so it shows a note icon from the start.

import { FileAudioIcon, FileImageIcon, FileMusicIcon, FileVideoIcon } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import type { FileKind, LibraryTile } from "@/api";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { formatBytes } from "@/lib/format";

export const KIND_ICON = {
  video: FileVideoIcon,
  image: FileImageIcon,
  audio: FileAudioIcon,
  midi: FileMusicIcon,
};

/** The kinds the server draws a thumbnail for; any other shows its icon. */
const PICTURED: ReadonlySet<FileKind> = new Set(["video", "image", "audio"]);

/** How often, and how many times, a thumbnail still being drawn is re-asked. */
const RETRY_MS = 4000;
const RETRIES = 8;

interface Props {
  tile: LibraryTile;
  selected: boolean;
  onSelect: () => void;
  onOpen: () => void;
  /** Shown greyed and not clickable — `note` says why. */
  disabled?: boolean;
  /** A word after the kind and size: "in this project", "can't go on a track". */
  note?: string;
  /** What the main button's hover text says it does. */
  hint?: string;
  /** Small buttons over the thumbnail's corner. */
  actions?: ReactNode;
}

export function FileTile({
  tile,
  selected,
  onSelect,
  onOpen,
  disabled,
  note,
  hint,
  actions,
}: Props) {
  const t = useT();
  const { language } = useLanguage();
  return (
    <div
      className={`relative flex flex-col overflow-hidden rounded-lg border transition-colors ${disabled ? "opacity-50" : "hover:bg-muted/50"} ${selected ? "border-primary ring-2 ring-primary/30" : ""}`}
    >
      <button
        type="button"
        onClick={onSelect}
        onDoubleClick={onOpen}
        disabled={disabled}
        title={hint}
        className="flex flex-col text-left disabled:cursor-not-allowed"
      >
        <Thumbnail src={tile.thumbnail} kind={tile.kind} />
        <span className="truncate px-2 pt-2 text-sm font-medium" title={tile.name}>
          {tile.name}
        </span>
        <span className="px-2 pb-2 text-xs text-muted-foreground">
          {t.files.kinds[tile.kind]} · {formatBytes(tile.size_bytes, language)}
          {note ? ` · ${note}` : ""}
        </span>
      </button>
      {actions && <div className="absolute top-1 right-1 flex gap-1">{actions}</div>}
    </div>
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
      {missing || !PICTURED.has(kind) ? (
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
