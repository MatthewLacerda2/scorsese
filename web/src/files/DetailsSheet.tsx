// A file's details panel: what it is, where it is used, what the assistant
// reads about it, and — for a generated file — how it was made and what it
// cost. Rename, describe, download and delete happen here; a delete a project
// blocks is refused by the server with the projects' names, shown as links.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { ApiError, api, type LibraryItem, type ProjectRef } from "@/api";
import { useLibraryItem } from "@/app/queries";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { formatBytes, formatDate, formatDuration } from "@/lib/format";
import { ConfirmDialog } from "./ConfirmDialog";
import { DownloadButton } from "./Download";
import { EditableText } from "./EditableText";
import { KIND_LABEL } from "./FileTile";
import { GenerationRecord } from "./GenerationRecord";
import type { Opened } from "./Viewer";

interface Props {
  id: number | null;
  onClose: () => void;
  onOpen: (file: Opened) => void;
}

export function DetailsSheet({ id, onClose, onOpen }: Props) {
  const item = useLibraryItem(id);
  return (
    <Sheet open={id !== null} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        {item.isError && (
          <SheetHeader>
            <SheetTitle>Not found</SheetTitle>
            <SheetDescription>{item.error.message}</SheetDescription>
          </SheetHeader>
        )}
        {item.data && <Details item={item.data} onOpen={onOpen} onDeleted={onClose} />}
      </SheetContent>
    </Sheet>
  );
}

function Details({
  item,
  onOpen,
  onDeleted,
}: {
  item: LibraryItem;
  onOpen: (file: Opened) => void;
  onDeleted: () => void;
}) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["library"] });
  const update = useMutation({
    mutationFn: (change: { name?: string; description?: string }) =>
      api.library.update(item.id, change),
    onSuccess: refresh,
  });
  const [confirming, setConfirming] = useState(false);
  const remove = useMutation({
    mutationFn: () => api.library.remove(item.id),
    onSuccess: () => {
      setConfirming(false);
      onDeleted();
      refresh();
    },
  });
  const blockedBy = inUseBy(remove.error);

  return (
    <div className="flex flex-col gap-6 px-4 pb-6">
      <SheetHeader className="px-0">
        <SheetTitle className="sr-only">{item.name}</SheetTitle>
        <EditableText
          label="Name"
          value={item.name}
          onSave={(name) => update.mutateAsync({ name })}
          className="text-lg font-semibold"
        />
        <SheetDescription>
          {KIND_LABEL[item.kind]}
          {item.generated ? " · generated" : ""}
        </SheetDescription>
      </SheetHeader>

      <div className="grid grid-cols-2 gap-2">
        <Button variant="secondary" onClick={() => onOpen(item)}>
          {item.kind === "image" ? "View" : item.kind === "midi" ? "Open" : "Play"}
        </Button>
        <DownloadButton file={item} />
      </div>

      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">Size</dt>
        <dd>{formatBytes(item.size_bytes)}</dd>
        {item.media.width !== undefined && item.media.height !== undefined && (
          <>
            <dt className="text-muted-foreground">Dimensions</dt>
            <dd>
              {item.media.width} × {item.media.height}
            </dd>
          </>
        )}
        {item.media.duration_seconds !== undefined && (
          <>
            <dt className="text-muted-foreground">Duration</dt>
            <dd>{formatDuration(item.media.duration_seconds)}</dd>
          </>
        )}
        <dt className="text-muted-foreground">Added</dt>
        <dd>{formatDate(item.created_at)}</dd>
      </dl>

      <section className="flex flex-col gap-1">
        <h3 className="text-sm font-medium">Description</h3>
        <EditableText
          label="Description"
          multiline
          value={item.description ?? ""}
          placeholder="Words the assistant reads when choosing a file"
          onSave={(description) => update.mutateAsync({ description })}
        />
      </section>
      {update.isError && <p className="text-sm text-destructive">{update.error.message}</p>}

      <section className="flex flex-col gap-1">
        <h3 className="text-sm font-medium">Used in</h3>
        <ProjectLinks projects={item.used_by} empty="No project uses it yet." />
        {item.templates.length > 0 && (
          <p className="text-sm text-muted-foreground">
            {`Templates: ${item.templates.map((template) => template.name).join(", ")}`}
          </p>
        )}
      </section>

      {item.generation && <GenerationRecord record={item.generation} />}

      <section className="flex flex-col gap-2">
        <Button variant="destructive" onClick={() => setConfirming(true)}>
          Delete
        </Button>
      </section>
      <ConfirmDialog
        open={confirming}
        onOpenChange={(open) => {
          setConfirming(open);
          if (!open) remove.reset();
        }}
        title={`Delete “${item.name}”?`}
        description="The file is removed from your library for good."
        confirm="Delete"
        busy={remove.isPending}
        onConfirm={() => remove.mutate()}
      >
        {remove.isError && <p className="text-sm text-destructive">{remove.error.message}</p>}
        {blockedBy && blockedBy.length > 0 && <ProjectLinks projects={blockedBy} empty="" />}
      </ConfirmDialog>
    </div>
  );
}

/** The projects a `409` names as keeping a file, or `null` for any other error. */
export function inUseBy(error: unknown): ProjectRef[] | null {
  if (!(error instanceof ApiError) || error.status !== 409) return null;
  const projects = error.body.projects;
  return Array.isArray(projects) ? (projects as ProjectRef[]) : null;
}

function ProjectLinks({ projects, empty }: { projects: ProjectRef[]; empty: string }) {
  if (projects.length === 0) return <p className="text-sm text-muted-foreground">{empty}</p>;
  return (
    <ul className="text-sm">
      {projects.map((project) => (
        <li key={project.id}>
          <Link to={`/projects/${project.id}`} className="underline">
            {project.name}
          </Link>
        </li>
      ))}
    </ul>
  );
}
