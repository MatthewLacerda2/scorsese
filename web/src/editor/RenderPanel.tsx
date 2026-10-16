// Delivering the video: ask for a finished render, watch it queue and run,
// download it (#541). A render is made by the job queue from the project as it
// is when asked, costs no credits, and an unchanged cut in the same shape
// answers at once with the file already made. What it is asked to be — the
// kind of file, its size and rate, the soundtrack's loudness — is `delivery`'s.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { DownloadIcon, FilmIcon, SquareIcon } from "lucide-react";
import { useState } from "react";
import { api, describeError, type Fps, type RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { queryKeys } from "@/app/queryKeys";
import { rememberRender } from "@/app/RenderToasts";
import { Button } from "@/components/ui/button";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { formatBytes, formatDate, formatNumber } from "@/lib/format";
import {
  CONTAINERS,
  type Container,
  LOUDNESS,
  type Loudness,
  RATES,
  renderAsk,
  soundOnly,
} from "./delivery";
import { folded, JobProgressBar } from "./JobProgressBar";
import { deliveries } from "./shape";

interface Props {
  projectId: number;
  /** The project's name, for the toast its render raises on another page. */
  name: string;
  /** The project's frame shape: the sizes offered are in it, the default first. */
  aspect: string;
  /** The project's own rate, the one offered first. */
  fps: Fps;
}

export function RenderPanel({ projectId, name, aspect, fps }: Props) {
  const queryClient = useQueryClient();
  const t = useT();
  const { language } = useLanguage();
  const key = queryKeys.renders(projectId);
  const renders = useQuery({ queryKey: key, queryFn: () => api.renders.list(projectId) });
  const sizes = deliveries(aspect);
  const [resolution, setResolution] = useState<string>(sizes[0]);
  // A size from before the shape changed is not offered any more.
  const chosen = sizes.includes(resolution) ? resolution : sizes[0];
  const [container, setContainer] = useState<Container>(CONTAINERS[0]);
  const [rate, setRate] = useState<string | null>(null);
  const [loudness, setLoudness] = useState<Loudness>("mixed");
  const sound = soundOnly(container);
  const own = formatNumber(Math.round((fps.num / fps.den) * 100) / 100, {}, language);
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
      const ask = renderAsk({ container, resolution: chosen, fps: rate, loudness });
      const asked = await api.renders.request(projectId, ask);
      if (asked.job) rememberRender(asked.job.id, { id: projectId, name });
      setJob(asked.job);
      if (asked.render) queryClient.invalidateQueries({ queryKey: key });
    } catch (failed) {
      setError(describeError(failed));
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
      setError(describeError(failed));
    }
  };
  const working = job?.state === "waiting" || job?.state === "running";

  return (
    <div className="flex flex-col gap-3 text-sm">
      <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2">
        <span className="text-xs text-muted-foreground">{t.editor.render.file}</span>
        <NativeSelect
          aria-label={t.editor.render.file}
          className="w-full"
          value={container}
          onChange={(event) => setContainer(event.target.value as Container)}
        >
          {CONTAINERS.map((kind) => (
            <NativeSelectOption key={kind} value={kind}>
              {soundOnly(kind) ? t.editor.render.sound(kind) : t.editor.render.video(kind)}
            </NativeSelectOption>
          ))}
        </NativeSelect>
        {/* A sound-only file has no picture to size or time, so neither is offered. */}
        {!sound && (
          <>
            <span className="text-xs text-muted-foreground">{t.editor.render.resolution}</span>
            <NativeSelect
              aria-label={t.editor.render.resolution}
              className="w-full"
              value={chosen}
              onChange={(event) => setResolution(event.target.value)}
            >
              {sizes.map((size) => (
                <NativeSelectOption key={size} value={size}>
                  {size}
                </NativeSelectOption>
              ))}
            </NativeSelect>
            <span className="text-xs text-muted-foreground">{t.editor.render.rate}</span>
            <NativeSelect
              aria-label={t.editor.render.rate}
              className="w-full"
              value={rate ?? ""}
              onChange={(event) => setRate(event.target.value || null)}
            >
              <NativeSelectOption value="">{t.editor.render.ownRate(own)}</NativeSelectOption>
              {RATES.map((each) => (
                <NativeSelectOption key={each} value={each}>
                  {t.editor.render.fps(each)}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </>
        )}
        <span className="text-xs text-muted-foreground">{t.editor.render.loudness}</span>
        <NativeSelect
          aria-label={t.editor.render.loudness}
          className="w-full"
          value={loudness}
          onChange={(event) => setLoudness(event.target.value as Loudness)}
        >
          {(Object.keys(LOUDNESS) as Loudness[]).map((each) => (
            <NativeSelectOption key={each} value={each}>
              {t.editor.render.loudnesses[each]}
            </NativeSelectOption>
          ))}
        </NativeSelect>
      </div>
      <Button size="sm" className="self-end" onClick={render}>
        <FilmIcon /> {t.editor.render.render}
      </Button>
      {job && (
        <div className="flex items-center gap-2">
          <div className="flex flex-1 flex-col gap-1">
            <p className="text-xs">
              {t.editor.render.jobs[job.state] ?? job.state}
              {job.error && job.state !== "cancelled" ? `: ${job.error}` : ""}
            </p>
            <JobProgressBar job={job} />
          </div>
          {working && (
            <Button size="sm" variant="ghost" onClick={stop}>
              <SquareIcon /> {t.editor.render.stop}
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
          <li className="text-xs text-muted-foreground">{t.editor.render.none}</li>
        )}
      </ul>
      <p className="text-xs text-muted-foreground">{t.editor.render.note}</p>
    </div>
  );
}

function Kept({ render }: { render: RenderView }) {
  const t = useT();
  const { language } = useLanguage();
  return (
    <li className="flex items-center gap-2 rounded-md border px-2 py-1.5">
      <span className="flex-1 text-xs">
        {render.settings.resolution ?? t.editor.render.audio} · .{render.settings.container} ·{" "}
        {formatBytes(render.size, language)}
        <span className="block text-muted-foreground">
          {formatDate(render.created_at, language)}
        </span>
      </span>
      <Button asChild size="icon" variant="ghost" aria-label={t.editor.render.download}>
        <a href={render.file} download>
          <DownloadIcon />
        </a>
      </Button>
    </li>
  );
}
