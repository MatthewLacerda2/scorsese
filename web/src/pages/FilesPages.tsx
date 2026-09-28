// The two views of files (#544): everything in the library, and the files
// one project uses — the same browser, narrowed by `?project=` on the server.

import { useQuery } from "@tanstack/react-query";
import { ArrowLeftIcon } from "lucide-react";
import { Link, useParams } from "react-router";
import { api } from "@/api";
import { FileBrowser } from "@/files/FileBrowser";

export function LibraryPage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="font-heading text-2xl font-semibold">Library</h1>
      <FileBrowser />
    </div>
  );
}

export function ProjectFilesPage() {
  const id = Number(useParams().id);
  const project = useQuery({
    queryKey: ["projects", id],
    queryFn: () => api.projects.open(id),
    enabled: Number.isInteger(id),
  });
  if (!Number.isInteger(id) || project.isError) {
    return (
      <p className="text-muted-foreground">
        {project.error?.message ?? "There is no such project."}{" "}
        <Link to="/projects" className="underline">
          Back to projects
        </Link>
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <Link
          to="/projects"
          className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeftIcon className="size-4" /> Projects
        </Link>
        <h1 className="font-heading text-2xl font-semibold">{project.data?.name ?? "…"}</h1>
        <Link to={`/projects/${id}/edit`} className="text-sm underline">
          Open it in the editor
        </Link>
        <p className="text-sm text-muted-foreground">
          The files from your library this project uses. Upload new ones in the{" "}
          <Link to="/library" className="underline">
            library
          </Link>
          .
        </p>
      </div>
      <FileBrowser project={id} />
    </div>
  );
}
