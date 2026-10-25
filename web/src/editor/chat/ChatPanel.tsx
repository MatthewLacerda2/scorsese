// The assistant's panel in the editor (#545 over #540's API): the project's
// current conversation, live while a turn runs, and a box to write the next
// message. The stored conversation is read once and on every `resync`; what a
// running turn does in between arrives on the event stream and is folded in by
// `transcript.ts`, which is where the logic lives and is tested. Above it, the
// model the project's assistant runs on (`ModelPicker.tsx`, #705) and, beside
// it, how hard it thinks (`EffortPicker.tsx`, #1443), both kept on the project,
// then the project's script, read and edited in a modal (`ScriptButton.tsx`, #1445).
// Beside them, "Full" (#1027): the editor gives the chat the whole page,
// and this panel draws its column at a readable width. It is the same mounted
// panel in both modes, so a draft, a running turn or an open picker outlives
// the switch.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Maximize2Icon, Minimize2Icon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { chatApi } from "@/api/chat";
import type { ServerEvent } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import { QueryError } from "@/components/ui/query-error";
import { ScriptButton } from "@/editor/script/ScriptButton";
import { useT } from "@/i18n/I18nProvider";
import type { Attachments } from "@/prompt/attach/useAttachments";
import type { Draft } from "@/prompt/draft";
import { Composer } from "./Composer";
import { EffortPicker } from "./EffortPicker";
import { effortOf } from "./effort";
import { Generating } from "./Generating";
import { ModelPicker } from "./ModelPicker";
import { Turn } from "./Turn";
import {
  apply,
  asking,
  empty,
  fromConversation,
  generating,
  restart,
  running,
  upsert,
} from "./transcript";

/** Full mode's column: wide enough to read, never lines across the whole screen. */
const COLUMN = "mx-auto w-full max-w-3xl";

/** How far from the bottom still counts as "at the bottom", in pixels. */
const NEAR_BOTTOM = 24;

export function ChatPanel({
  projectId,
  full,
  onFull,
  attachments,
  draft,
}: {
  projectId: number;
  /** A first message the project was opened with but could not send (#1535). */
  draft?: Draft;
  /** The files attached to the next message: the page takes a drop anywhere (#1534). */
  attachments: Attachments;
  /** Whether the chat has the whole page. */
  full: boolean;
  onFull: (full: boolean) => void;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const conversation = useQuery({
    queryKey: queryKeys.conversation(projectId),
    queryFn: () => chatApi.conversation(projectId),
  });
  const [transcript, setTranscript] = useState(() => empty(projectId));

  useEffect(() => {
    if (conversation.data) {
      const stored = conversation.data;
      setTranscript((previous) => fromConversation(stored, previous));
    }
  }, [conversation.data]);

  useServerEvents((event: ServerEvent) => {
    if (event.type === "resync") {
      conversation.refetch();
      return;
    }
    // The header's balance is the same money the turn just spent.
    if (event.type === "chat_turn") queryClient.invalidateQueries({ queryKey: queryKeys.credits });
    setTranscript((previous) => apply(previous, event));
  });

  const current = running(transcript);
  const paused = asking(transcript);
  const scroller = useRef<HTMLDivElement>(null);
  // Follow new text only while the reader is at the bottom; scrolled up, leave them be (#1437).
  const atBottom = useRef(true);
  const watch = () => {
    const box = scroller.current;
    if (box) atBottom.current = box.scrollHeight - box.scrollTop - box.clientHeight <= NEAR_BOTTOM;
  };
  // biome-ignore lint/correctness/useExhaustiveDependencies: scroll whenever the transcript grows
  useEffect(() => {
    if (atBottom.current) scroller.current?.scrollTo({ top: scroller.current.scrollHeight });
  }, [transcript]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b px-3 py-2">
        <div className={`flex items-start gap-2 ${full ? COLUMN : ""}`}>
          <div className="min-w-0 flex-1">
            {conversation.data && (
              <ModelPicker
                projectId={projectId}
                model={conversation.data.model}
                models={conversation.data.models}
                turns={transcript.entries.map((entry) => entry.turn)}
              />
            )}
          </div>
          {conversation.data && (
            <EffortPicker projectId={projectId} effort={effortOf(conversation.data.effort)} />
          )}
          <ScriptButton projectId={projectId} labelled={full} />
          {/* Beside the picker in a narrow sidebar, the icon alone; named either way. */}
          <Button
            size={full ? "sm" : "icon-sm"}
            variant="ghost"
            className="shrink-0"
            aria-label={full ? t.chat.full.leave : t.chat.full.enter}
            title={full ? t.chat.full.leave : t.chat.full.enter}
            onClick={() => onFull(!full)}
          >
            {full ? <Minimize2Icon /> : <Maximize2Icon />}
            {full && t.chat.full.leave}
          </Button>
        </div>
      </div>
      <div
        ref={scroller}
        onScroll={watch}
        className="flex min-h-0 flex-1 flex-col overflow-y-auto p-3"
      >
        <div className={`flex flex-col gap-4 ${full ? COLUMN : ""}`}>
          {conversation.isPending && (
            <p className="text-sm text-muted-foreground">{t.chat.loading}</p>
          )}
          <QueryError error={conversation.error} />
          <ol className="flex flex-col gap-5">
            {transcript.entries.map((entry) => (
              <Turn key={entry.turn.id} entry={entry} />
            ))}
          </ol>
          {generating(transcript) && <Generating />}
        </div>
      </div>
      <Composer
        projectId={projectId}
        column={full ? COLUMN : ""}
        runningTurn={current?.id ?? null}
        askingTurn={paused?.id ?? null}
        fresh={transcript.fresh}
        onFresh={() => setTranscript(restart)}
        showFresh={transcript.entries.length > 0}
        onStarted={(turn) => setTranscript((previous) => upsert(previous, turn))}
        attachments={attachments}
        draft={draft}
      />
    </div>
  );
}
