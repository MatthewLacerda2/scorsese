// The Drive-like view of files, used twice: the whole library, and one
// project's files (`project` set). Flat by design (#527): filter by kind,
// search by name, sort — no folders. A click selects a file and opens its
// details; a double click, or "Open" in the details, views or plays it.
//
// The selected file lives in the URL (`?item=12`), so a link from elsewhere —
// the spending history, "you already have this" — opens it directly.

import { UploadIcon } from "lucide-react";
import { type DragEvent, useDeferredValue, useRef, useState } from "react";
import { useSearchParams } from "react-router";
import { FILE_KINDS, type FileKind } from "@/api";
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
import { DetailsSheet } from "./DetailsSheet";
import { FileTile, KIND_LABEL } from "./FileTile";
import { SORTS, type Sort, sortTiles } from "./sort";
import { useUploads } from "./uploads";
import { type Opened, Viewer } from "./Viewer";

const ALL = "all";

export function FileBrowser({ project }: { project?: number }) {
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
    <section aria-label="Files" className="flex min-h-[50vh] flex-col gap-4" {...drop}>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          placeholder="Search by name"
          aria-label="Search by name"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          className="w-full sm:w-64"
        />
        <Select value={kind} onValueChange={(value) => setKind(value as FileKind | typeof ALL)}>
          <SelectTrigger aria-label="Kind" className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={ALL}>All kinds</SelectItem>
            {FILE_KINDS.map((k) => (
              <SelectItem key={k} value={k}>
                {KIND_LABEL[k]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={sort} onValueChange={(value) => setSort(value as Sort)}>
          <SelectTrigger aria-label="Sort" className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(SORTS).map(([value, label]) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {project === undefined && <UploadButton />}
      </div>

      {files.isError && <p className="text-destructive">{files.error.message}</p>}
      {files.isPending && <p className="text-muted-foreground">Loading…</p>}
      {files.isSuccess && tiles.length === 0 && (
        <p className="text-muted-foreground">
          {filtered
            ? "No files match."
            : project === undefined
              ? "Your library is empty. Upload videos, pictures, sounds and MIDI files — or drop them here — to use in any project."
              : "This project uses no files from your library yet."}
        </p>
      )}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6">
        {tiles.map((tile) => (
          <FileTile
            key={tile.id}
            tile={tile}
            selected={tile.id === selected}
            onSelect={() => select(tile.id)}
            onOpen={() => setOpened(tile)}
          />
        ))}
      </div>

      <DetailsSheet id={selected} onClose={() => select(null)} onOpen={setOpened} />
      <Viewer file={opened} onClose={() => setOpened(null)} />
    </section>
  );
}

/** Pick files and send them to the uploader; the tray shows how they go. */
function UploadButton() {
  const { add } = useUploads();
  const input = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button className="ml-auto" onClick={() => input.current?.click()}>
        <UploadIcon /> Upload
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
