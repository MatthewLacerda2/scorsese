// The preview's moving picture — the doctrine #542 records in docs/web.md,
// *The editor*. The server renders the cut small, at the chosen quality and
// from proxies of heavy videos, and the browser plays it in an ordinary video
// element, where scrubbing is seeking. The browser does no compositing.
//
// **Made only when play is pressed** (#1448). Nothing is rendered after an
// edit by itself: a shared machine's CPU is not spent on cuts nobody watches.
// A preview is keyed by the document and its settings, so pressing play on an
// unchanged cut answers at once with the file already made. A queued preview
// of a revision the project has moved past retires when its turn comes.
//
// This is the cut played **whole**, for one of thirty seconds or less; a
// longer one plays in pieces from the playhead (`pieces/`).
//
// Tied to a revision: a render of revision 7 says nothing about revision 8,
// so the moment the project moves the video is dropped and the preview falls
// back to stills until the next one is ready.

import { useCallback, useEffect, useState } from "react";
import { api, type RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { useServerEvents } from "@/app/events";
import type { Messages } from "@/i18n/catalogue";
import { folded } from "../JobProgressBar";
import type { Quality } from "./quality";

/** Where the preview video is, for the revision on screen. */
type Playable =
  | { state: "none" }
  | { state: "preparing"; job: JobView }
  | { state: "ready"; render: RenderView }
  // Never the server's sentence (#1439): that is for the job row and the
  // log, and "Broken pipe (os error 32)" under a picture helps nobody. The
  // editor says the preview could not be made, in its own words.
  | { state: "failed"; why: null };

/** How often a render on its way is asked about, in case no event arrives. */
const POLL_MS = 4000;

interface Held {
  revision: number;
  playable: Playable;
}

/** Say what a job's state means to someone waiting for their preview. */
export function waiting(job: JobView, t: Messages["editor"]["preview"]): string {
  if (job.state === "waiting") return t.queued;
  if (job.state === "running") return t.rendering;
  return t.ready;
}

/**
 * The preview video of project `id` at `revision`, delivered at `deliver` and
 * drawn at `quality`: asked for by `ask`, when play is pressed.
 */
export function usePlayable(id: number, revision: number, deliver: string, quality: Quality) {
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
          : // Neither a render nor a job: nothing more to say than that it failed.
            { state: "failed", why: null };
      setHeld({ revision: at, playable: next });
    } catch (error) {
      console.error("preview: asking failed", error);
      setHeld({ revision: at, playable: { state: "failed", why: null } });
    }
  }, [id, revision, deliver, quality]);

  useServerEvents((event) => {
    if (playable.state !== "preparing" || !("id" in event) || event.id !== playable.job.id) return;
    if (event.type === "job_progress") {
      setHeld({ revision, playable: { state: "preparing", job: folded(playable.job, event) } });
      return;
    }
    if (event.type !== "job") return;
    // Done — or retired as superseded, or stopped by a newer preview: asking
    // again for the revision on screen answers correctly in every case.
    if (event.state === "done" || event.state === "cancelled") void ask();
    else if (event.state === "failed" || event.state === "stuck") {
      console.error(`preview: job ${event.id} ${event.state}`, event.error);
      setHeld({ revision, playable: { state: "failed", why: null } });
    } else
      setHeld({ revision, playable: { state: "preparing", job: folded(playable.job, event) } });
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
