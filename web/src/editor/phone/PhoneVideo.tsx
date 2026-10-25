// The editor's video view on a phone (#1570): the preview at the width of the
// screen, and the timeline under it filling the rest. Editing on a phone is the
// chat's, so the timeline here only scrubs: its ruler and its empty lanes seek,
// and nothing on it is selected, dragged, trimmed or removed. The timeline is
// drawn as it is everywhere, and this view takes its hands off it: a clip lets
// a press through to its lane (which seeks), and a press on any of its buttons
// (a clip, a lane's bin) never reaches them.

import type { PointerEvent } from "react";
import type { ProjectDocument } from "@/api";
import { ErrorBoundary } from "@/app/ErrorBoundary";
import { Preview } from "../preview/Preview";
import { Timeline } from "../timeline/Timeline";

interface Props {
  projectId: number;
  revision: number;
  document: ProjectDocument;
  playhead: number;
  onSeek: (frame: number) => void;
  /** The size the preview is drawn at, `WIDTHxHEIGHT`. */
  deliver: string;
}

const nothing = () => {};

/** A press on one of the timeline's buttons, stopped before it gets there. */
function untouched(event: PointerEvent) {
  if (event.target instanceof Element && event.target.closest("button")) event.stopPropagation();
}

export function PhoneVideo({ projectId, revision, document, playhead, onSeek, deliver }: Props) {
  const [width = 16, height = 9] = deliver.split("x").map(Number);
  // The picture at the screen's width under its padding, plus the transport;
  // a tall frame stops short of the whole screen so the timeline keeps a share.
  const tall = `min(calc((100vw - 1.5rem) * ${height / width} + 4.5rem), 65%)`;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <section className="shrink-0 border-b" style={{ height: tall }}>
        <ErrorBoundary place="panel">
          <Preview
            projectId={projectId}
            revision={revision}
            document={document}
            playhead={playhead}
            onSeek={onSeek}
            deliver={deliver}
          />
        </ErrorBoundary>
      </section>
      <section
        data-scrub-only=""
        className="min-h-0 flex-1 [&_[data-slot=button]]:hidden [&_[data-track]>button]:pointer-events-none"
        onPointerDownCapture={untouched}
      >
        <ErrorBoundary place="panel">
          <Timeline
            document={document}
            playhead={playhead}
            onSeek={onSeek}
            selected={[]}
            onSelect={nothing}
            onDeselect={nothing}
            onRelease={async () => {}}
            onDrop={nothing}
            onRemoveTrack={nothing}
          />
        </ErrorBoundary>
      </section>
    </div>
  );
}
