// The files one project uses (#544): the library's browser, narrowed by
// `?project=` on the server, under the project's name and a way back.

import { useQuery } from "@tanstack/react-query";
import { ArrowLeftIcon } from "lucide-react";
import { Link, useParams } from "react-router";
import { api } from "@/api";
import { queryKeys } from "@/app/queryKeys";
import { FileBrowser } from "@/files/FileBrowser";
import { useT } from "@/i18n/I18nProvider";

export function ProjectFilesPage() {
  const t = useT();
  const words = t.pages.projectFiles;
  const id = Number(useParams().id);
  const project = useQuery({
    queryKey: queryKeys.project(id),
    queryFn: () => api.projects.open(id),
    enabled: Number.isInteger(id),
  });
  if (!Number.isInteger(id) || project.isError) {
    return (
      <p className="text-muted-foreground">
        {project.error?.message ?? words.noSuchProject}{" "}
        <Link to="/projects" className="underline">
          {words.back}
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
          <ArrowLeftIcon className="size-4" /> {t.common.nav.projects}
        </Link>
        <h1 className="font-heading text-2xl font-semibold">{project.data?.name ?? "…"}</h1>
        <Link to={`/projects/${id}/edit`} className="text-sm underline">
          {words.openEditor}
        </Link>
        <p className="text-sm text-muted-foreground">
          {words.intro}{" "}
          <Link to="/library" className="underline">
            {words.libraryLink}
          </Link>
          {words.introEnd}
        </p>
      </div>
      <FileBrowser project={id} />
    </div>
  );
}
