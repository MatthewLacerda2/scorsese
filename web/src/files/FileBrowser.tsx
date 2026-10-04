// The Drive-like view of files, used twice: the whole library, and one
// project's files (`project` set). Flat by design (#527): filter by kind,
// search by name, sort — no folders. A click selects a file and opens its
// details; a double click, or "Open" in the details, views or plays it.
//
// The selected file lives in the URL (`?item=12`), so a link from elsewhere —
// the spending history, "you already have this" — opens it directly.
//
// With `picking` it is the editor's "add from your library" modal (#702):
// the same grid, search and uploads, but a click picks the file instead of
// opening its details, and the URL is left alone — it is the editor's.

import { UploadIcon } from "lucide-react";
import { type DragEvent, useDeferredValue, useRef, useState } from "react";
import { useSearchParams } from "react-router";
import { FILE_KINDS, type FileKind, type LibraryTile } from "@/api";
import { useLibrary } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n/I18nProvider";
import { DetailsSheet } from "./DetailsSheet";
import { DownloadAction } from "./Download";
import { FileTile } from "./FileTile";
import { SORTS, type Sort, sortTiles } from "./sort";
import { useUploads } from "./uploads";
import { type Opened, Viewer } from "./Viewer";

const ALL = "all";

/** The browser as a picker: what a click does, and which files it refuses. */
export interface Picking {
  pick: (tile: LibraryTile) => void;
  /** Why a file cannot be picked, or `null` when it can. */
  refuse: (tile: LibraryTile) => string | null;
  /** The kinds the filter offers. */
  kinds: readonly FileKind[];
}

interface Props {
  /** Only the files this project uses. */
  project?: number;
  picking?: Picking;
}

export function FileBrowser({ project, picking }: Props) {
  const t = useT();
  const words = t.files.browser;
  const [kind, setKind] = useState<FileKind | typeof ALL>(ALL);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<Sort>("newest");
  const [opened, setOpened] = useState<Opened | null>(null);
  const [params, setParams] = useSearchParams();
  const { add } = useUploads();
  const deferredSearch = useDeferredValue(search.trim());
  const files = useLibrary({
    kind: kind === ALL ? undefined : kind,
    search: deferredSearch || undefined,
    project,
  });

  const selected = Number(params.get("item")) || null;
  const select = (id: number | null) =>
    setParams(
      (current) => {
        const next = new URLSearchParams(current);
        if (id === null) next.delete("item");
        else next.set("item", String(id));
        return next;
      },
      { replace: true },
    );

  const tiles = sortTiles(files.data ?? [], sort);
  const filtered = kind !== ALL || deferredSearch !== "";

  // Files dropped anywhere on the library upload, as they do in Drive. Only
  // the library: a project's view lists what the project uses, which an
  // upload does not change.
  const drop =
    project === undefined
      ? {
          onDragOver: (event: DragEvent) => event.preventDefault(),
          onDrop: (event: DragEvent) => {
            event.preventDefault();
            void add(Array.from(event.dataTransfer.files));
          },
        }
      : {};

  return (
    <section aria-label={words.label} className="flex min-h-[50vh] flex-col gap-4" {...drop}>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          placeholder={words.search}
          aria-label={words.search}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          className="w-full sm:w-64"
        />
        <Select value={kind} onValueChange={(value) => setKind(value as FileKind | typeof ALL)}>
          <SelectTrigger aria-label={words.kind} className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>{words.allKinds}</SelectItem>
            {(picking?.kinds ?? FILE_KINDS).map((k) => (
              <SelectItem key={k} value={k}>
                {t.files.kinds[k]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={sort} onValueChange={(value) => setSort(value as Sort)}>
          <SelectTrigger aria-label={words.sort} className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SORTS.map((value) => (
              <SelectItem key={value} value={value}>
                {t.files.sorts[value]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {project === undefined && <UploadButton />}
      </div>

      {files.isError && <p className="text-destructive">{files.error.message}</p>}
      {files.isPending && <p className="text-muted-foreground">{t.common.loading}</p>}
      {files.isSuccess && tiles.length === 0 && (
        <p className="text-muted-foreground">
          {filtered
            ? words.noMatch
            : project === undefined
              ? words.emptyLibrary
              : words.emptyProject}
        </p>
      )}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6">
        {tiles.map((tile) =>
          picking ? (
            <PickTile key={tile.id} tile={tile} picking={picking} />
          ) : (
            <FileTile
              key={tile.id}
              tile={tile}
              selected={tile.id === selected}
              onSelect={() => select(tile.id)}
              onOpen={() => setOpened(tile)}
              actions={<DownloadAction file={tile} />}
            />
          ),
        )}
      </div>

      {!picking && (
        <>
          <DetailsSheet id={selected} onClose={() => select(null)} onOpen={setOpened} />
          <Viewer file={opened} onClose={() => setOpened(null)} />
        </>
      )}
    </section>
  );
}

/** A tile in picking mode: a click picks it, unless the picker refuses it. */
function PickTile({ tile, picking }: { tile: LibraryTile; picking: Picking }) {
  const t = useT();
  const refused = picking.refuse(tile);
  return (
    <FileTile
      tile={tile}
      selected={false}
      onSelect={() => picking.pick(tile)}
      onOpen={() => {}}
      disabled={refused !== null}
      note={refused ?? undefined}
      hint={refused === null ? t.files.browser.add(tile.name) : undefined}
      // Saving a file is not picking it, and a file the picker refuses is
      // still the user's to save.
      actions={<DownloadAction file={tile} />}
    />
  );
}

/** Pick files and send them to the uploader; the tray shows how they go. */
function UploadButton() {
  const t = useT();
  const { add } = useUploads();
  const input = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button className="ml-auto" onClick={() => input.current?.click()}>
        <UploadIcon /> {t.files.browser.upload}
      </Button>
      <input
        ref={input}
        type="file"
        multiple
        hidden
        // A hint for the picker; the server holds files to exactly the
        // extensions `scorsese import` takes, and MIDI, and says so when it
        // refuses one.
        accept="video/*,image/*,audio/*,.mid,.midi"
        onChange={(event) => {
          const picked = Array.from(event.target.files ?? []);
          event.target.value = "";
          void add(picked);
        }}
      />
    </>
  );
}
