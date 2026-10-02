// The preview's moving picture — the doctrine #542 records in docs/web.md,
// *The editor*. The server renders the cut small, at the chosen quality and
// from proxies of heavy videos, and the browser plays it in an ordinary video
// element, where scrubbing is seeking. The browser does no compositing.
//
// **Re-rendered after every edit, without a press.** Once the revision has
// been still for a moment the preview of it is asked for; a preview is keyed
// by the document and its settings, so an unchanged cut answers at once with
// the file already made. Edits faster than renders are the server's to
// absorb: a queued preview of a revision the project has moved past retires
// when its turn comes, and the newest one replaces the older ones.
//
// Tied to a revision: a render of revision 7 says nothing about revision 8,
// so the moment the project moves the video is dropped and the preview falls
// back to stills until the next one is ready.

import { useCallback, useEffect, useState } from "react";
import { api, type RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { useServerEvents } from "@/app/events";
import type { Quality } from "./quality";

/** Where the preview video is, for the revision on screen. */
export type Playable =
  | { state: "none" }
  | { state: "preparing"; job: JobView }
  | { state: "ready"; render: RenderView }
  | { state: "failed"; why: string };

/** How often a render on its way is asked about, in case no event arrives. */
const POLL_MS = 4000;

/** How long the revision rests before its preview is asked for. */
const SETTLE_MS = 1000;

interface Held {
  revision: number;
  playable: Playable;
}

/** Say what a job's state means to someone waiting for their preview. */
export function waiting(job: JobView): string {
  if (job.state === "waiting") return "Preview queued…";
  if (job.state === "running") return "Rendering the preview…";
  return "Preview ready";
}

/**
 * The preview video of project `id` at `revision`, delivered at `deliver` and
 * drawn at `quality`. Asked for by itself once the revision rests, when
 * `wanted` (there is something on the timeline); `ask` asks now.
 */
export function usePlayable(
  id: number,
  revision: number,
  deliver: string,
  quality: Quality,
  wanted: boolean,
) {
  const [held, setHeld] = useState<Held>({ revision, playable: { state: "none" } });
  const playable: Playable = held.revision === revision ? held.playable : { state: "none" };

  const ask = useCallback(async () => {
    const at = revision;
    try {
      const asked = await api.renders.preview(id, { resolution: deliver, quality });
      const next: Playable = asked.render
        ? { state: "ready", render: asked.render }
        : asked.job
          ? { state: "preparing", job: asked.job }
          : { state: "failed", why: "the server answered with neither a render nor a job" };
      setHeld({ revision: at, playable: next });
    } catch (error) {
      setHeld({ revision: at, playable: { state: "failed", why: (error as Error).message } });
    }
  }, [id, revision, deliver, quality]);

  // A new revision, shape or quality: once it has rested, its preview.
  useEffect(() => {
    if (!wanted) return;
    const timer = setTimeout(() => void ask(), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [wanted, ask]);

  useServerEvents((event) => {
    if (event.type !== "job" || playable.state !== "preparing" || event.id !== playable.job.id)
      return;
    // Done — or retired as superseded, or stopped by a newer preview: asking
    // again for the revision on screen answers correctly in every case.
    if (event.state === "done" || event.state === "cancelled") void ask();
    else if (event.state === "failed" || event.state === "stuck") {
      setHeld({ revision, playable: { state: "failed", why: event.error ?? "the render failed" } });
    } else
      setHeld({ revision, playable: { state: "preparing", job: { ...playable.job, ...event } } });
  });

  // Asked again now and then as well: the event stream does not pass every
  // tunnel (docs/web.md, *Testing before there is a domain*), and asking about
  // a preview on its way never queues a second one.
  const preparing = playable.state === "preparing";
  useEffect(() => {
    if (!preparing) return;
    const timer = setInterval(() => void ask(), POLL_MS);
    return () => clearInterval(timer);
  }, [preparing, ask]);

  return { playable, ask };
}
