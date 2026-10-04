// The assistant's panel in the editor (#545 over #540's API): the project's
// current conversation, live while a turn runs, and a box to write the next
// message. The stored conversation is read once and on every `resync`; what a
// running turn does in between arrives on the event stream and is folded in by
// `transcript.ts`, which is where the logic lives and is tested. Above it, the
// model the project's assistant runs on (`ModelPicker.tsx`, #705).

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { SendIcon, SquareIcon } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { chatApi } from "@/api/chat";
import type { ServerEvent } from "@/api/events";
import { useServerEvents } from "@/app/events";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { ModelPicker } from "./ModelPicker";
import { Turn } from "./Turn";
import {
  apply,
  asking,
  empty,
  fromConversation,
  jobState,
  type Problem,
  problem,
  running,
  upsert,
} from "./transcript";

export function ChatPanel({ projectId }: { projectId: number }) {
  const t = useT();
  const queryClient = useQueryClient();
  const conversation = useQuery({
    queryKey: ["chat", projectId],
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
    if (event.type === "chat_turn") queryClient.invalidateQueries({ queryKey: ["credits"] });
    setTranscript((previous) => apply(previous, event));
  });

  const current = running(transcript);
  const paused = asking(transcript);
  const scroller = useRef<HTMLDivElement>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: scroll whenever the transcript grows
  useEffect(() => {
    scroller.current?.scrollTo({ top: scroller.current.scrollHeight });
  }, [transcript]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      {conversation.data && (
        <div className="border-b px-3 py-2">
          <ModelPicker
            projectId={projectId}
            model={conversation.data.model}
            models={conversation.data.models}
            turns={transcript.entries.map((entry) => entry.turn)}
          />
        </div>
      )}
      <div ref={scroller} className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-3">
        {conversation.isPending && (
          <p className="text-sm text-muted-foreground">{t.chat.loading}</p>
        )}
        {conversation.isError && (
          <p className="text-sm text-destructive">{conversation.error.message}</p>
        )}
        <ol className="flex flex-col gap-5">
          {transcript.entries.map((entry) => (
            <Turn key={entry.turn.id} entry={entry} />
          ))}
        </ol>
        {transcript.jobs.length > 0 && (
          <ul className="flex flex-col gap-0.5 border-t pt-2 text-xs text-muted-foreground">
            {transcript.jobs.map((job) => (
              <li key={job.id}>
                {(t.chat.jobs as Record<string, string>)[job.kind] ?? job.kind} #{job.id}:{" "}
                {jobState(job, t.chat)}
              </li>
            ))}
          </ul>
        )}
      </div>
      <Composer
        projectId={projectId}
        runningTurn={current?.id ?? null}
        askingTurn={paused?.id ?? null}
        onStarted={(turn) => setTranscript((previous) => upsert(previous, turn))}
      />
    </div>
  );
}

/**
 * The box a message is written in, with Send, Stop and "new conversation".
 * While a turn waits on a question, a message sent here is its answer, and
 * Stop sets the question aside.
 */
function Composer({
  projectId,
  runningTurn,
  askingTurn,
  onStarted,
}: {
  projectId: number;
  runningTurn: number | null;
  askingTurn: number | null;
  onStarted: (turn: Awaited<ReturnType<typeof chatApi.send>>) => void;
}) {
  const t = useT();
  const [prompt, setPrompt] = useState("");
  const [fresh, setFresh] = useState(false);
  const [refused, setRefused] = useState<Problem | null>(null);
  const send = useMutation({
    mutationFn: () => chatApi.send(projectId, prompt.trim(), fresh),
    onMutate: () => setRefused(null),
    onSuccess: (turn) => {
      setPrompt("");
      setFresh(false);
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
    <div className="flex flex-col gap-2 border-t p-3">
      {refused && (
        <div
          className={`rounded-md px-2 py-1.5 text-xs ${refused.tone === "note" ? "bg-muted text-muted-foreground" : "bg-destructive/10 text-destructive"}`}
        >
          {refused.lead && <p className="font-medium">{refused.lead}</p>}
          <p>{refused.detail}</p>
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
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            checked={fresh}
            onChange={(event) => setFresh(event.target.checked)}
          />
          {t.chat.composer.fresh}
        </label>
        <div className="ml-auto flex gap-2">
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
  );
}
