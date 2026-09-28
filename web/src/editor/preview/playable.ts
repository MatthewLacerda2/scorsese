// The preview's moving picture: the cut rendered small by the server, played
// in an ordinary video element (the doctrine #542 records — the browser does
// no compositing). A render is keyed by the document and its settings (#541),
// so asking again for an unchanged cut answers at once with the file already
// made, and asking after an edit queues a new one.
//
// Tied to a revision: a render of revision 7 says nothing about revision 8,
// so the moment the project moves the video is dropped and the preview falls
// back to stills until the next render is asked for.

import { useCallback, useEffect, useState } from "react";
import { api, type RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { useServerEvents } from "@/app/events";

/** Where the preview video is, for the revision on screen. */
export type Playable =
  | { state: "none" }
  | { state: "preparing"; job: JobView }
  | { state: "ready"; render: RenderView }
  | { state: "failed"; why: string };

/** How often a render on its way is asked about, in case no event arrives. */
const POLL_MS = 4000;

interface Held {
  revision: number;
  playable: Playable;
}

/** Say what a job's state means to someone waiting for their preview. */
export function waiting(job: JobView): string {
  if (job.state === "waiting") return "Preview queued behind other renders…";
  if (job.state === "running") return "Rendering the preview…";
  return "Preview ready";
}

export function usePlayable(id: number, revision: number, resolution: string) {
  const [held, setHeld] = useState<Held>({ revision, playable: { state: "none" } });
  const playable: Playable = held.revision === revision ? held.playable : { state: "none" };

  const ask = useCallback(async () => {
    const at = revision;
    try {
      const asked = await api.renders.request(id, { resolution });
      const next: Playable = asked.render
        ? { state: "ready", render: asked.render }
        : asked.job
          ? { state: "preparing", job: asked.job }
          : { state: "failed", why: "the server answered with neither a render nor a job" };
      setHeld({ revision: at, playable: next });
    } catch (error) {
      setHeld({ revision: at, playable: { state: "failed", why: (error as Error).message } });
    }
  }, [id, revision, resolution]);

  useServerEvents((event) => {
    if (event.type !== "job" || playable.state !== "preparing" || event.id !== playable.job.id)
      return;
    if (event.state === "done") void ask();
    else if (event.state === "failed" || event.state === "stuck") {
      setHeld({ revision, playable: { state: "failed", why: event.error ?? "the render failed" } });
    } else
      setHeld({ revision, playable: { state: "preparing", job: { ...playable.job, ...event } } });
  });

  // Asked again now and then as well: the event stream does not pass every
  // tunnel (docs/web.md, *Testing before there is a domain*), and asking about
  // a render on its way never queues a second one.
  const preparing = playable.state === "preparing";
  useEffect(() => {
    if (!preparing) return;
    const timer = setInterval(() => void ask(), POLL_MS);
    return () => clearInterval(timer);
  }, [preparing, ask]);

  return { playable, ask };
}
