// The user's projects: make one, open one, rename or delete one. Opening a
// project opens the editor (#545); its files are a click away.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ExternalLinkIcon, FolderOpenIcon, PlusIcon, Trash2Icon } from "lucide-react";
import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, type ProjectSummary } from "@/api";
import { useProjects } from "@/app/queries";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ConfirmDialog } from "@/files/ConfirmDialog";
import { EditableText } from "@/files/EditableText";
import { formatDate } from "@/lib/format";

export function ProjectsPage() {
  const projects = useProjects();
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <h1 className="font-heading text-2xl font-semibold">Projects</h1>
      <NewProject />
      {projects.isError && <p className="text-destructive">{projects.error.message}</p>}
      {projects.isPending && <p className="text-muted-foreground">Loading…</p>}
      {projects.data?.length === 0 && (
        <p className="text-muted-foreground">No projects yet. Name one above to start.</p>
      )}
      <ul className="flex flex-col divide-y rounded-lg border">
        {projects.data?.map((project) => (
          <ProjectRow key={project.id} project={project} />
        ))}
      </ul>
    </div>
  );
}

function NewProject() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const create = useMutation({
    mutationFn: api.projects.create,
    onSuccess: (project) => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      navigate(`/projects/${project.id}/edit`);
    },
  });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (name.trim()) create.mutate(name.trim());
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <div className="flex gap-2">
        <Input
          aria-label="New project's name"
          placeholder="A new project's name"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <Button type="submit" disabled={!name.trim() || create.isPending}>
          <PlusIcon /> Create
        </Button>
      </div>
      {create.isError && <p className="text-sm text-destructive">{create.error.message}</p>}
    </form>
  );
}

function ProjectRow({ project }: { project: ProjectSummary }) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["projects"] });
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
          label="Project name"
          value={project.name}
          onSave={(name) => rename.mutateAsync(name)}
          className="font-medium"
        />
        <span className="text-xs text-muted-foreground">
          Changed {formatDate(project.updated_at)}
        </span>
        {rename.isError && <span className="text-xs text-destructive">{rename.error.message}</span>}
      </div>
      <Button asChild variant="success" size="sm">
        <Link to={`/projects/${project.id}/edit`}>
          <ExternalLinkIcon /> Open
        </Link>
      </Button>
      <Button asChild variant="outline" size="sm">
        <Link to={`/projects/${project.id}`}>
          <FolderOpenIcon /> Files
        </Link>
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={`Delete ${project.name}`}
        onClick={() => setConfirming(true)}
      >
        <Trash2Icon />
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Delete “${project.name}”?`}
        description="The project is deleted for good. Its files stay in your library, and what it cost stays in your spending history."
        confirm="Delete"
        busy={remove.isPending}
        onConfirm={() => remove.mutate()}
      >
        {remove.isError && <p className="text-sm text-destructive">{remove.error.message}</p>}
      </ConfirmDialog>
    </li>
  );
}
