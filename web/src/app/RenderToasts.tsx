// A toast when a render finishes (#958), wherever the person is in the app:
// "Your video is ready" with a Download link, or the failure in the words the
// render panel shows. One listener and one `<Toaster />`, mounted once by the
// shell, so it fires whichever page is open — the render panel only updates
// its own list, and a person who went back to the chat or another project
// would otherwise never know.
//
// Renders only: a preview is seconds long and would be noise, and a cancelled
// render is one the person stopped themselves. A job carries no project, so
// the panel remembers which project it asked for (`rememberRender`); a render
// asked elsewhere (the assistant, another tab) is still toasted, unnamed.

import type { ReactNode } from "react";
import { useMatch } from "react-router";
import { toast } from "sonner";
import type { ServerEvent } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import type { Messages } from "@/i18n/catalogue";
import { useT } from "@/i18n/I18nProvider";

/** A project a render was asked for in this tab. */
interface Asked {
  id: number;
  name: string;
}

const asked = new Map<number, Asked>();

/** Remember that render job `job` is of `project`, so its toast can name it. */
export function rememberRender(job: number, project: Asked): void {
  asked.set(job, project);
}

/** What to toast for `event`, or `null` for nothing. */
export interface RenderToast {
  tone: "success" | "error";
  title: string;
  description?: string | undefined;
  /** Where to download the video, once it is ready. */
  file?: string | undefined;
}

/**
 * The toast a finished render earns: its project named only when it is not
 * the one open (`open`), since there the person knows which video it is.
 */
export function renderToast(
  event: ServerEvent,
  open: number | null,
  t: Messages["editor"]["render"],
): RenderToast | null {
  if (event.type !== "job" || event.kind !== "render") return null;
  const project = asked.get(event.id);
  const named = project && project.id !== open ? project.name : undefined;
  switch (event.state) {
    case "done": {
      asked.delete(event.id);
      const file = event.result?.file;
      return {
        tone: "success",
        title: t.ready,
        description: named,
        file: typeof file === "string" ? file : undefined,
      };
    }
    case "failed":
    case "stuck": {
      asked.delete(event.id);
      const why = t.jobs[event.state] ?? event.state;
      return {
        tone: "error",
        title: named ? `${why} · ${named}` : why,
        description: event.error ?? undefined,
      };
    }
    default:
      return null;
  }
}

/** The toaster and its listener, told which project is open by the route. */
export function RenderToasts() {
  const editing = useMatch("/projects/:id/edit");
  return <RenderToaster open={editing ? Number(editing.params.id) : null} />;
}

/** The toaster, and the listener that feeds it finished renders. */
function RenderToaster({ open }: { open: number | null }) {
  const t = useT();
  useServerEvents((event) => {
    const shown = renderToast(event, open, t.editor.render);
    if (!shown) return;
    let action: ReactNode;
    if (shown.file) {
      action = (
        <Button asChild size="sm">
          <a href={shown.file} download>
            {t.editor.render.download}
          </a>
        </Button>
      );
    }
    toast[shown.tone](shown.title, { description: shown.description, action });
  });
  return <Toaster position="bottom-right" />;
}
