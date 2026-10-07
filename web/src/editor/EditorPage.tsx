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
import { XIcon } from "lucide-react";
import { useCallback, useState } from "react";
import { useParams } from "react-router";
import type { EditorProject } from "@/api";
import { useServerEvents } from "@/app/events";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { AssetsPanel } from "./assets/AssetsPanel";
import { ChatPanel } from "./chat/ChatPanel";
import { useDeleteKey } from "./deleting";
import { useDrop } from "./drop";
import { EditorHeader } from "./EditorHeader";
import { Inspector } from "./inspector/Inspector";
import { Handle } from "./panels/Handle";
import { usePanels } from "./panels/usePanels";
import { Preview } from "./preview/Preview";
import { editorKey, useEdit, useEditorProject } from "./project";
import { confirmThen, trackRemoval } from "./removing";
import { choose, kept } from "./selection";
import { SHAPES, type Shape, savedShape, saveShape } from "./shape";
import { Timeline } from "./timeline/Timeline";

export function EditorPage() {
  const id = Number(useParams().id);
  const project = useEditorProject(id);
  const t = useT();
  if (project.isError) return <p className="p-6 text-destructive">{project.error.message}</p>;
  if (!project.data) return <p className="p-6 text-muted-foreground">{t.editor.page.opening}</p>;
  return <Editor project={project.data} />;
}

function Editor({ project }: { project: EditorProject }) {
  const { id, revision, document } = project;
  const queryClient = useQueryClient();
  const t = useT();
  const edit = useEdit(id);
  const [playhead, setPlayhead] = useState(0);
  const [picked, setPicked] = useState<string[]>([]);
  const [shape, setShape] = useState<Shape>(() => savedShape(id));
  const panels = usePanels();
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
      <EditorHeader
        projectId={id}
        name={document.name}
        selected={selected}
        edit={edit}
        shape={shape}
        onShape={(next) => {
          setShape(next);
          saveShape(id, next);
        }}
      />
      {edit.refused && (
        <div className="flex items-start gap-2 border-b bg-destructive/10 px-3 py-1.5 text-sm text-destructive">
          <p className="flex-1">{edit.refused}</p>
          <Button
            size="icon"
            variant="ghost"
            aria-label={t.editor.page.dismiss}
            onClick={edit.dismiss}
          >
            <XIcon />
          </Button>
        </div>
      )}
      {/* Each panel's edge is a handle with a track of its own (#863). */}
      <div ref={panels.grid} className="grid min-h-0 flex-1" style={panels.style}>
        <aside className="col-start-1 row-start-1 min-h-0">
          <AssetsPanel projectId={id} document={document} edit={edit} playhead={playhead} />
        </aside>
        <Handle
          grows="right"
          size={panels.sizes.assets}
          label={t.editor.page.resizeAssets}
          className="col-start-2 row-start-1"
          onSize={(size) => panels.resize("assets", size)}
          onReset={() => panels.reset("assets")}
        />
        <section className="col-start-3 row-start-1 min-h-0">
          <Preview
            projectId={id}
            revision={revision}
            document={document}
            playhead={playhead}
            onSeek={setPlayhead}
            deliver={SHAPES[shape].deliver[0]}
          />
        </section>
        <Handle
          grows="left"
          size={panels.sizes.chat}
          label={t.editor.page.resizeChat}
          className="col-start-4 row-span-3 row-start-1"
          onSize={(size) => panels.resize("chat", size)}
          onReset={() => panels.reset("chat")}
        />
        <aside className="col-start-5 row-span-3 row-start-1 flex min-h-0 flex-col">
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
        <Handle
          grows="up"
          size={panels.sizes.timeline}
          label={t.editor.page.resizeTimeline}
          className="col-span-3 col-start-1 row-start-2"
          onSize={(size) => panels.resize("timeline", size)}
          onReset={() => panels.reset("timeline")}
        />
        <section className="col-span-3 col-start-1 row-start-3 min-h-0">
          <Timeline
            document={document}
            playhead={playhead}
            onSeek={setPlayhead}
            selected={selected}
            onSelect={(clip, adding) => setPicked((now) => choose(now, clip, adding))}
            onDeselect={deselect}
            onRelease={({ tool, args }) => edit.run({ tool, args, edit: true })}
            onDrop={(dragged, track, pointed, reach) => void drop(dragged, track, pointed, reach)}
            onRemoveTrack={(track) => confirmThen(trackRemoval(track, t.editor.removal), edit.run)}
          />
        </section>
      </div>
    </div>
  );
}
