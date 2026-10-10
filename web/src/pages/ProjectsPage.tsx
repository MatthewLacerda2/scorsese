// The user's projects: make one, open one, rename or delete one. Create sits
// beside the heading and opens the new-project modal (#1016); opening a
// project opens the editor (#545); its files are a click away.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ExternalLinkIcon, FolderOpenIcon, Trash2Icon } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "react-router";
import { api, type ProjectSummary } from "@/api";
import { EditorPage } from "@/app/pages";
import { useProjects } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/files/ConfirmDialog";
import { EditableText } from "@/files/EditableText";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { formatDate } from "@/lib/format";
import { NewProjectButton } from "./NewProjectDialog";

export function ProjectsPage() {
  const projects = useProjects();
  const t = useT();
  usePreloadEditor();
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <div className="flex items-center justify-between gap-3">
        <h1 className="font-heading text-2xl font-semibold">{t.common.nav.projects}</h1>
        <NewProjectButton />
      </div>
      {projects.isError && <p className="text-destructive">{projects.error.message}</p>}
      {projects.isPending && <p className="text-muted-foreground">{t.common.loading}</p>}
      {projects.data?.length === 0 && (
        <p className="text-muted-foreground">{t.pages.projects.empty}</p>
      )}
      <ul className="flex flex-col divide-y rounded-lg border">
        {projects.data?.map((project) => (
          <ProjectRow key={project.id} project={project} />
        ))}
      </ul>
    </div>
  );
}

/**
 * Fetch the editor's chunk once this page is idle, so opening a project from
 * here does not wait on a download (#896). Safari has no `requestIdleCallback`,
 * so there it is a short timeout instead.
 */
function usePreloadEditor() {
  useEffect(() => {
    const preload = () => void EditorPage.preload();
    if ("requestIdleCallback" in window) {
      const id = requestIdleCallback(preload);
      return () => cancelIdleCallback(id);
    }
    const id = setTimeout(preload, 1000);
    return () => clearTimeout(id);
  }, []);
}

function ProjectRow({ project }: { project: ProjectSummary }) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
  const t = useT();
  const words = t.pages.projects;
  const { language } = useLanguage();
  const rename = useMutation({
    mutationFn: (name: string) => api.projects.rename(project.id, name),
    onSuccess: refresh,
  });
  const [confirming, setConfirming] = useState(false);
  const remove = useMutation({
    mutationFn: () => api.projects.remove(project.id),
    onSuccess: () => {
      setConfirming(false);
      refresh();
    },
  });

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <div className="flex min-w-0 flex-1 flex-col">
        <EditableText
          label={words.name}
          value={project.name}
          onSave={(name) => rename.mutateAsync(name)}
          className="font-medium"
        />
        <span className="text-xs text-muted-foreground">
          {words.changed(formatDate(project.updated_at, language))}
        </span>
        {rename.isError && <span className="text-xs text-destructive">{rename.error.message}</span>}
      </div>
      <Button asChild variant="success" size="sm">
        <Link to={`/projects/${project.id}/edit`}>
          <ExternalLinkIcon /> {words.open}
        </Link>
      </Button>
      <Button asChild variant="outline" size="sm">
        <Link to={`/projects/${project.id}`}>
          <FolderOpenIcon /> {words.files}
        </Link>
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={words.deleteLabel(project.name)}
        onClick={() => setConfirming(true)}
      >
        <Trash2Icon />
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={words.deleteTitle(project.name)}
        description={words.deleteDescription}
        confirm={words.delete}
        busy={remove.isPending}
        onConfirm={() => remove.mutate()}
      >
        {remove.isError && <p className="text-sm text-destructive">{remove.error.message}</p>}
      </ConfirmDialog>
    </li>
  );
}
