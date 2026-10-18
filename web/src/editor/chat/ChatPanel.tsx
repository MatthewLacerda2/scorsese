// The assistant's panel in the editor (#545 over #540's API): the project's
// current conversation, live while a turn runs, and a box to write the next
// message. The stored conversation is read once and on every `resync`; what a
// running turn does in between arrives on the event stream and is folded in by
// `transcript.ts`, which is where the logic lives and is tested. Above it, the
// model the project's assistant runs on (`ModelPicker.tsx`, #705); beside
// Send, how hard it thinks on the next message (`EffortPicker.tsx`, #769).
// Beside the model, "Full" (#1027): the editor gives the chat the whole page,
// and this panel draws its column at a readable width. It is the same mounted
// panel in both modes, so a draft, a running turn or an open picker outlives
// the switch.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Maximize2Icon, Minimize2Icon, SendIcon, SquareIcon } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { chatApi } from "@/api/chat";
import type { ServerEvent } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { NoCredits } from "@/app/NoCredits";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import { QueryError } from "@/components/ui/query-error";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { browserStorage } from "@/lib/storage";
import { EffortPicker } from "./EffortPicker";
import { type Effort, saveEffort, storedEffort } from "./effort";
import { Generating } from "./Generating";
import { ModelPicker } from "./ModelPicker";
import { NewConversation } from "./NewConversation";
import { Turn } from "./Turn";
import {
  apply,
  asking,
  empty,
  fromConversation,
  generating,
  type Problem,
  problem,
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
  onBusy,
}: {
  projectId: number;
  /** Whether the chat has the whole page. */
  full: boolean;
  onFull: (full: boolean) => void;
  /** Told whether a turn is running — the assistant may be changing the project (#1448). */
  onBusy?: (busy: boolean) => void;
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
  const busy = current !== null;
  useEffect(() => onBusy?.(busy), [busy, onBusy]);
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
      />
    </div>
  );
}

/**
 * The box a message is written in, with Send, Stop and "new conversation" (#1437).
 * While a turn waits on a question, a message sent here is its answer, and
 * Stop sets the question aside.
 */
function Composer({
  projectId,
  column,
  runningTurn,
  askingTurn,
  fresh,
  onFresh,
  showFresh,
  onStarted,
}: {
  projectId: number;
  /** Extra classes for its content: full mode's column. */
  column: string;
  runningTurn: number | null;
  askingTurn: number | null;
  /** The next message opens a new conversation. */
  fresh: boolean;
  onFresh: () => void;
  /** There is a conversation to leave. */
  showFresh: boolean;
  onStarted: (turn: Awaited<ReturnType<typeof chatApi.send>>) => void;
}) {
  const t = useT();
  const [prompt, setPrompt] = useState("");
  const [effort, setEffort] = useState(() => storedEffort(browserStorage(), projectId));
  const choose = (level: Effort) => {
    setEffort(level);
    saveEffort(browserStorage(), projectId, level);
  };
  const [refused, setRefused] = useState<Problem | null>(null);
  const send = useMutation({
    mutationFn: () => chatApi.send(projectId, prompt.trim(), fresh, effort),
    onMutate: () => setRefused(null),
    onSuccess: (turn) => {
      setPrompt("");
      onStarted(turn);
    },
    onError: (error) => setRefused(problem(error, t.chat)),
  });
  const stop = useMutation({
    mutationFn: (turn: number) => chatApi.stop(turn),
    onError: (error) => setRefused(problem(error, t.chat)),
  });
  const ready = prompt.trim() !== "" && runningTurn === null && !send.isPending;
  const stoppable = runningTurn ?? askingTurn;
  const keyed = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (ready) send.mutate();
    }
  };
  return (
    <div className="border-t p-3">
      <div className={`flex flex-col gap-2 ${column}`}>
        {refused && (
          <div
            className={`rounded-md px-2 py-1.5 text-xs ${refused.tone === "note" ? "bg-muted text-muted-foreground" : "bg-destructive/10 text-destructive"}`}
          >
            {refused.lead && <p className="font-medium">{refused.lead}</p>}
            <p>{refused.detail}</p>
            {refused.noCredit && <NoCredits always />}
          </div>
        )}
        <Textarea
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          onKeyDown={keyed}
          placeholder={
            askingTurn !== null && !fresh
              ? t.chat.composer.answerPlaceholder
              : t.chat.composer.placeholder
          }
          className="max-h-40 min-h-16 resize-none"
        />
        <div className="flex items-center gap-2">
          {showFresh && !fresh && runningTurn === null && <NewConversation onStart={onFresh} />}
          <div className="ml-auto flex gap-2">
            <EffortPicker effort={effort} onChange={choose} />
            {stoppable !== null && (
              <Button
                size="sm"
                variant="outline"
                disabled={stop.isPending}
                onClick={() => stop.mutate(stoppable)}
              >
                <SquareIcon /> {t.chat.composer.stop}
              </Button>
            )}
            <Button size="sm" disabled={!ready} onClick={() => send.mutate()}>
              <SendIcon /> {t.chat.composer.send}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
