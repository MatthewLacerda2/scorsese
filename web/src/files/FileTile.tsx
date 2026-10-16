// One file in the grid: thumbnail (`Thumbnail.tsx`) and name, without its
// extension. A tile can be `disabled` with a `note` saying why (the editor's
// picking modal, #702), and carries `actions` beside its main button — buttons
// of their own, since a button cannot hold one.

import type { ReactNode } from "react";
import type { LibraryTile } from "@/api";
import { Thumbnail } from "./Thumbnail";

/** `name` without its last extension; a dotfile or an extensionless name is whole. */
export function withoutExtension(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

interface Props {
  tile: LibraryTile;
  selected: boolean;
  onSelect: () => void;
  onOpen: () => void;
  /** Shown greyed and not clickable — `note` says why. */
  disabled?: boolean;
  /** A line under the name: "in this project", "can't go on a track". */
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
  return (
    <div
      className={`relative flex flex-col overflow-hidden rounded-lg border transition-colors ${disabled ? "opacity-50" : "hover:bg-muted/50"} ${selected ? "border-primary ring-2 ring-primary/30" : ""}`}
    >
      {/* biome-ignore lint/plugin/shadcnControls: the whole tile face (thumbnail and name) is one click target; a Button's fixed height and centring would fight the layout */}
      <button
        type="button"
        onClick={onSelect}
        onDoubleClick={onOpen}
        disabled={disabled}
        title={hint}
        className="flex flex-col text-left disabled:cursor-not-allowed"
      >
        <Thumbnail src={tile.thumbnail} kind={tile.kind} />
        <span
          className={`truncate px-2 pt-2 text-sm font-medium ${note ? "" : "pb-2"}`}
          title={tile.name}
        >
          {withoutExtension(tile.name)}
        </span>
        {note && <span className="px-2 pb-2 text-xs text-muted-foreground">{note}</span>}
      </button>
      {actions && <div className="absolute top-1 right-1 flex gap-1">{actions}</div>}
    </div>
  );
}
