// Delivering the video: ask for a finished render, watch it queue and run,
// download it (#541). A render is made by the job queue from the project as it
// is when asked, costs no credits, and an unchanged cut in the same size
// answers at once with the file already made.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { DownloadIcon, FilmIcon, SquareIcon } from "lucide-react";
import { useState } from "react";
import { api, type RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { Button } from "@/components/ui/button";
import { formatBytes, formatDate } from "@/lib/format";
import { folded, JobProgressBar } from "./JobProgressBar";
import { SHAPES, type Shape } from "./shape";

const JOB_WORDS: Record<string, string> = {
  waiting: "Queued behind other renders…",
  running: "Rendering…",
  done: "Ready — below.",
  failed: "The render failed",
  stuck: "The render is stuck",
  cancelled: "Stopped — nothing was kept",
};

export function RenderPanel({ projectId, shape }: { projectId: number; shape: Shape }) {
  const queryClient = useQueryClient();
  const key = ["projects", "renders", projectId];
  const renders = useQuery({ queryKey: key, queryFn: () => api.renders.list(projectId) });
  const sizes = SHAPES[shape].deliver;
  const [resolution, setResolution] = useState<string>(sizes[0] ?? "1920x1080");
  const [job, setJob] = useState<JobView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useServerEvents((event) => {
    if (!job || (event.type !== "job" && event.type !== "job_progress") || event.id !== job.id)
      return;
    setJob(folded(job, event));
    if (event.type === "job" && event.state === "done")
      queryClient.invalidateQueries({ queryKey: key });
  });

  const render = async () => {
    setError(null);
    try {
      const asked = await api.renders.request(projectId, { resolution });
      setJob(asked.job);
      if (asked.render) queryClient.invalidateQueries({ queryKey: key });
    } catch (failed) {
      setError((failed as Error).message);
    }
  };

  // Stopping answers with the job as it is now; a running render then turns
  // `cancelled` on the event stream within a frame.
  const stop = async () => {
    if (!job) return;
    setError(null);
    try {
      setJob(await api.jobs.cancel(job.id));
    } catch (failed) {
      setError((failed as Error).message);
    }
  };
  const working = job?.state === "waiting" || job?.state === "running";

  return (
    <div className="flex flex-col gap-3 text-sm">
      <div className="flex gap-2">
        <select
          aria-label="Resolution"
          className="h-8 flex-1 rounded-md border bg-transparent px-2"
          value={sizes.includes(resolution) ? resolution : sizes[0]}
          onChange={(event) => setResolution(event.target.value)}
        >
          {sizes.map((size) => (
            <option key={size} value={size}>
              {size} · MP4
            </option>
          ))}
        </select>
        <Button size="sm" onClick={render}>
          <FilmIcon /> Render
        </Button>
      </div>
      {job && (
        <div className="flex items-center gap-2">
          <div className="flex flex-1 flex-col gap-1">
            <p className="text-xs">
              {JOB_WORDS[job.state] ?? job.state}
              {job.error && job.state !== "cancelled" ? `: ${job.error}` : ""}
            </p>
            <JobProgressBar job={job} />
          </div>
          {working && (
            <Button size="sm" variant="ghost" onClick={stop}>
              <SquareIcon /> Stop
            </Button>
          )}
        </div>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
      <ul className="flex flex-col gap-1">
        {renders.data?.map((kept) => (
          <Kept key={kept.id} render={kept} />
        ))}
        {renders.data?.length === 0 && (
          <li className="text-xs text-muted-foreground">No renders kept for this project yet.</li>
        )}
      </ul>
      <p className="text-xs text-muted-foreground">
        Renders are kept for a while and made again on request; they cost no credits.
      </p>
    </div>
  );
}

function Kept({ render }: { render: RenderView }) {
  return (
    <li className="flex items-center gap-2 rounded-md border px-2 py-1.5">
      <span className="flex-1 text-xs">
        {render.settings.resolution ?? "audio"} · {formatBytes(render.size)}
        <span className="block text-muted-foreground">{formatDate(render.created_at)}</span>
      </span>
      <Button asChild size="icon" variant="ghost" aria-label="Download">
        <a href={render.file} download>
          <DownloadIcon />
        </a>
      </Button>
    </li>
  );
}
