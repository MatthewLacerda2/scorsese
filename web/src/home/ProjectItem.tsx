// One project in the sidebar: its name, a link to the editor, and a ⋯ menu
// (shown on hover, always on touch screens and while open) with Rename and
// Delete. Rename turns the row into a field in place: Enter saves, Escape puts
// the old name back, and leaving the field saves a changed name and drops an
// unchanged one. Delete asks first, in the same dialog the projects list uses.

import { MoreHorizontalIcon, PencilIcon, Trash2Icon } from "lucide-react";
import { type KeyboardEvent, useRef, useState } from "react";
import { Link } from "react-router";
import { api, type ProjectSummary } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { useInvalidatingMutation } from "@/app/useInvalidatingMutation";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { QueryError } from "@/components/ui/query-error";
import { ConfirmDialog } from "@/files/ConfirmDialog";
import { useT } from "@/i18n/I18nProvider";

export function ProjectItem({
  project,
  onNavigate,
}: {
  project: ProjectSummary;
  onNavigate?: () => void;
}) {
  const t = useT();
  const words = t.pages.projects;
  const [renaming, setRenaming] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [draft, setDraft] = useState(project.name);
  const finished = useRef(false);
  const rename = useInvalidatingMutation([queryKeys.projects], {
    mutationFn: (name: string) => api.projects.rename(project.id, name),
  });
  const remove = useInvalidatingMutation([queryKeys.projects], {
    mutationFn: () => api.projects.remove(project.id),
    onSuccess: () => setConfirming(false),
  });

  const start = () => {
    finished.current = false;
    setDraft(project.name);
    rename.reset();
    setRenaming(true);
  };
  const finish = (save: boolean) => {
    if (finished.current) return;
    finished.current = true;
    setRenaming(false);
    const name = draft.trim();
    if (save && name && name !== project.name) rename.mutate(name);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter") finish(true);
    if (event.key === "Escape") finish(false);
  };

  return (
    <li className="group/project">
      {renaming ? (
        <Input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={onKeyDown}
          onBlur={() => finish(true)}
          aria-label={words.name}
          autoFocus
          onFocus={(event) => event.currentTarget.select()}
          className="h-8"
        />
      ) : (
        <div className="relative flex items-center rounded-lg hover:bg-muted has-[[aria-expanded=true]]:bg-muted">
          <Link
            to={`/projects/${project.id}/edit`}
            onClick={onNavigate}
            className="min-w-0 flex-1 truncate rounded-lg px-2.5 py-1.5 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            {project.name}
          </Link>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={t.home.options(project.name)}
                className="mr-1 opacity-100 hover:bg-background md:opacity-0 md:group-hover/project:opacity-100 md:focus-visible:opacity-100 aria-expanded:opacity-100"
              >
                <MoreHorizontalIcon />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onSelect={start}>
                <PencilIcon /> {t.home.rename}
              </DropdownMenuItem>
              <DropdownMenuItem variant="destructive" onSelect={() => setConfirming(true)}>
                <Trash2Icon /> {t.home.delete}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
      <QueryError error={rename.error} className="px-2.5 text-xs text-destructive" />
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={words.deleteTitle(project.name)}
        description={words.deleteDescription}
        confirm={words.delete}
        busy={remove.isPending}
        onConfirm={() => remove.mutate()}
      >
        <QueryError error={remove.error} />
      </ConfirmDialog>
    </li>
  );
}
