// The box a message is written in, under the chat (#1437), with the files
// attached to it above the text (#1534): the "+" here, or a drop anywhere on
// the editor's page, puts them in its tray, and a message carries their
// library ids, so the server brings them into the project and names them to
// the assistant — no trip to the assets panel's Library first.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { SendIcon, SquareIcon } from "lucide-react";
import { type KeyboardEvent, useState } from "react";
import { chatApi } from "@/api/chat";
import { NoCredits } from "@/app/NoCredits";
import { queryKeys } from "@/app/queryKeys";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useT } from "@/i18n/I18nProvider";
import { AttachButton } from "@/prompt/attach/AttachButton";
import { AttachmentTray } from "@/prompt/attach/AttachmentTray";
import type { Attachments } from "@/prompt/attach/useAttachments";
import { NewConversation } from "./NewConversation";
import { type Problem, problem } from "./transcript";

/**
 * The box a message is written in, with Send, Stop and "new conversation" (#1437).
 * While a turn waits on a question, a message sent here is its answer, and
 * Stop sets the question aside.
 */
export function Composer({
  projectId,
  column,
  runningTurn,
  askingTurn,
  fresh,
  onFresh,
  showFresh,
  onStarted,
  attachments,
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
  attachments: Attachments;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const [prompt, setPrompt] = useState("");
  const [refused, setRefused] = useState<Problem | null>(null);
  const send = useMutation({
    mutationFn: () => chatApi.send(projectId, prompt.trim(), fresh, attachments.items),
    onMutate: () => setRefused(null),
    onSuccess: (turn) => {
      setPrompt("");
      // The files are the project's now: the assets panel lists them.
      if (attachments.items.length > 0)
        queryClient.invalidateQueries({ queryKey: queryKeys.library });
      attachments.clear();
      onStarted(turn);
    },
    onError: (error) => setRefused(problem(error, t.chat)),
  });
  const stop = useMutation({
    mutationFn: (turn: number) => chatApi.stop(turn),
    onError: (error) => setRefused(problem(error, t.chat)),
  });
  const ready =
    prompt.trim() !== "" && runningTurn === null && !send.isPending && !attachments.busy;
  const stoppable = runningTurn ?? askingTurn;
  const keyed = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (ready) send.mutate();
    }
  };
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a hint to ask what the "+" takes, which is itself the keyboard way to attach
    <div
      className="border-t p-3"
      onPointerEnter={attachments.prepare}
      onFocus={attachments.prepare}
    >
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
        <AttachmentTray attachments={attachments} className="" />
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
          <AttachButton attachments={attachments} className="size-7" />
          {showFresh && !fresh && runningTurn === null && <NewConversation onStart={onFresh} />}
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
    </div>
  );
}
