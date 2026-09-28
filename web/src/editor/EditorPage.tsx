// The editor, `/projects/:id/edit` (#545), laid out as the desktop app is: the
// assets on the left, the preview in the middle, the timeline under them, and
// on the right the selected clip's inspector over the assistant's chat.
//
// The page is thin on purpose (CLAUDE.md, *The GUI is thin*): the hand-edits
// are place, move (along a lane or onto another), trim, delete, a plain value,
// and saving the selected clips as a template or putting one in (#546), each
// one a call to the server's tools; anything with structure to it is a sentence to the assistant. What
// the page draws is always the server's document — after an edit from its
// answer, after the assistant's from the `project` event.

import { useQueryClient } from "@tanstack/react-query";
import { ArrowLeftIcon, FilmIcon, XIcon } from "lucide-react";
import { useCallback, useState } from "react";
import { Link, useParams } from "react-router";
import type { EditorProject } from "@/api";
import { useServerEvents } from "@/app/events";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { AssetsPanel } from "./assets/AssetsPanel";
import { ChatPanel } from "./chat/ChatPanel";
import { useDeleteKey } from "./deleting";
import { useDrop } from "./drop";
import { Inspector } from "./inspector/Inspector";
import { Preview } from "./preview/Preview";
import { editorKey, useEdit, useEditorProject } from "./project";
import { RenderPanel } from "./RenderPanel";
import { choose, kept } from "./selection";
import { SHAPES, type Shape, savedShape, saveShape } from "./shape";
import { SaveTemplate } from "./templates/SaveTemplate";
import { Timeline } from "./timeline/Timeline";

export function EditorPage() {
  const id = Number(useParams().id);
  const project = useEditorProject(id);
  if (project.isError) return <p className="p-6 text-destructive">{project.error.message}</p>;
  if (!project.data) return <p className="p-6 text-muted-foreground">Opening the project…</p>;
  return <Editor project={project.data} />;
}

function Editor({ project }: { project: EditorProject }) {
  const { id, revision, document } = project;
  const queryClient = useQueryClient();
  const edit = useEdit(id);
  const [playhead, setPlayhead] = useState(0);
  const [picked, setPicked] = useState<string[]>([]);
  const [shape, setShape] = useState<Shape>(() => savedShape(id));
  const drop = useDrop(id, edit, playhead, (clip) => setPicked([clip]));
  const deselect = useCallback(() => setPicked([]), []);

  // The assistant, another tab or a finished generation changed the project:
  // draw what is there now, and the preview follows its revision.
  useServerEvents((event) => {
    const moved = event.type === "project" && event.id === id && event.revision !== revision;
    if (moved || event.type === "resync")
      queryClient.invalidateQueries({ queryKey: editorKey(id) });
  });

  const tracks = document.tracks ?? [];
  const found = tracks.flatMap((track) => track.clips.map((clip) => ({ track, clip })));
  // Only clips the project still has: the assistant may have removed one.
  const selected = kept(picked, new Set(found.map(({ clip }) => clip.id)));
  useDeleteKey(selected, edit.run, deselect);
  const chosen =
    selected.length === 1 ? found.find(({ clip }) => clip.id === selected[0]) : undefined;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center gap-2 border-b px-3 py-1.5">
        <Button asChild size="icon" variant="ghost" aria-label="Back to projects">
          <Link to="/projects">
            <ArrowLeftIcon />
          </Link>
        </Button>
        <h1 className="truncate font-heading font-semibold">{document.name}</h1>
        <span className="text-xs text-muted-foreground">{`revision ${revision}`}</span>
        <div className="ml-auto flex items-center gap-2">
          <SaveTemplate clips={selected} edit={edit} />
          <select
            aria-label="Frame shape"
            className="h-8 rounded-md border bg-transparent px-2 text-sm"
            value={shape}
            onChange={(event) => {
              const next = event.target.value as Shape;
              setShape(next);
              saveShape(id, next);
            }}
          >
            {(Object.keys(SHAPES) as Shape[]).map((choice) => (
              <option key={choice} value={choice}>
                {SHAPES[choice].label}
              </option>
            ))}
          </select>
          <Dialog>
            <DialogTrigger asChild>
              <Button size="sm" variant="outline">
                <FilmIcon /> Render
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-sm">
              <DialogHeader>
                <DialogTitle>Render the video</DialogTitle>
                <DialogDescription>
                  The whole cut as it is now, as an MP4 to download.
                </DialogDescription>
              </DialogHeader>
              <RenderPanel projectId={id} shape={shape} />
            </DialogContent>
          </Dialog>
        </div>
      </div>
      {edit.refused && (
        <div className="flex items-start gap-2 border-b bg-destructive/10 px-3 py-1.5 text-sm text-destructive">
          <p className="flex-1">{edit.refused}</p>
          <Button size="icon" variant="ghost" aria-label="Dismiss" onClick={edit.dismiss}>
            <XIcon />
          </Button>
        </div>
      )}
      <div className="grid min-h-0 flex-1 grid-cols-[15rem_minmax(0,1fr)_22rem] grid-rows-[minmax(0,1fr)_15rem]">
        <aside className="min-h-0 border-r">
          <AssetsPanel document={document} edit={edit} playhead={playhead} />
        </aside>
        <section className="min-h-0">
          <Preview
            projectId={id}
            revision={revision}
            document={document}
            playhead={playhead}
            onSeek={setPlayhead}
            deliver={SHAPES[shape].deliver[0]}
          />
        </section>
        <aside className="row-span-2 flex min-h-0 flex-col border-l">
          {chosen && (
            <div className="max-h-[55%] shrink-0 overflow-y-auto border-b">
              <Inspector
                clip={chosen.clip}
                track={chosen.track}
                asset={document.assets?.find((asset) => asset.id === chosen.clip.asset)}
                fps={document.timeline_fps}
                onChange={(tool, args) => void edit.run({ tool, args, edit: true })}
              />
            </div>
          )}
          <div className="min-h-0 flex-1">
            <ChatPanel projectId={id} />
          </div>
        </aside>
        <section className="col-span-2 min-h-0 border-t">
          <Timeline
            document={document}
            playhead={playhead}
            onSeek={setPlayhead}
            selected={selected}
            onSelect={(clip, adding) => setPicked((now) => choose(now, clip, adding))}
            onDeselect={deselect}
            onRelease={({ tool, args }) => edit.run({ tool, args, edit: true })}
            onDrop={(dragged, track, pointed, reach) => void drop(dragged, track, pointed, reach)}
            onAddTrack={(kind) => void edit.run({ tool: "track_new", args: { kind }, edit: true })}
          />
        </section>
      </div>
    </div>
  );
}
