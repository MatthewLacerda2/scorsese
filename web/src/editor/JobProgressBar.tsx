// How far a render or preview has got (#698): a bar with the percentage, and
// the phase in words wherever the frames say nothing — 0% while the sound is
// mixed and 99% while the file is finished would otherwise read as stuck.
// The server pushes `job_progress` at most twice a second and folds the same
// figures into a job read while it runs, so a reload or a poll shows it too.

import type { JobProgress, JobView, ServerEvent } from "@/api/events";
import { Progress } from "@/components/ui/progress";
import type { Messages } from "@/i18n/catalogue";
import { useT } from "@/i18n/I18nProvider";

/** What a running job is doing, in words: `42%`, `0% · mixing the sound…`. */
export function progressWords(progress: JobProgress, t: Messages["editor"]["progress"]): string {
  const percent = `${progress.percent}%`;
  switch (progress.phase) {
    case "preparing":
      return t.preparing(percent);
    case "mixing":
      return t.mixing(percent);
    case "finishing":
      return t.finishing(percent);
    default:
      return percent;
  }
}

/** `job` as `event` leaves it: a state change replaces it, progress joins it. */
export function folded(job: JobView, event: ServerEvent): JobView {
  if (event.type === "job_progress" && event.id === job.id) {
    return job.state === "running" ? { ...job, progress: event.progress } : job;
  }
  if (event.type === "job" && event.id === job.id) {
    const { type: _, ...changed } = event;
    // A state change carries no progress; a job still running keeps its own.
    return changed.state === "running" ? { ...changed, progress: job.progress } : changed;
  }
  return job;
}

/** The bar for a running job that reports progress; nothing otherwise. */
export function JobProgressBar({ job }: { job: JobView }) {
  const t = useT();
  const progress = job.state === "running" ? job.progress : null;
  if (!progress) return null;
  const words = progressWords(progress, t.editor.progress);
  return (
    <div className="flex w-full items-center gap-2">
      <Progress value={progress.percent} aria-label={words} className="flex-1" />
      <span className="shrink-0 text-xs tabular-nums text-muted-foreground">{words}</span>
    </div>
  );
}
