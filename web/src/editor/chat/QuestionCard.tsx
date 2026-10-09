// The assistant's question mid-edit (#710): one short question, two to four
// options and a field for the user's own answer. Either resumes the same turn
// (`POST /api/chat/turns/{id}/answer`), and so does a message written in the
// composer instead — the card is the quick way, never the only one. Once
// answered, the card shrinks to the question and what was answered.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { chatApi, type QuestionView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useT } from "@/i18n/I18nProvider";
import { typedAnswer } from "./question";

export function QuestionCard({
  turn,
  question,
  waiting,
}: {
  turn: number;
  question: QuestionView;
  /** Whether this is the question the turn is paused on now. */
  waiting: boolean;
}) {
  const t = useT();
  if (!waiting) {
    return (
      <div className="flex flex-col gap-0.5 rounded-lg border px-3 py-2 text-xs">
        <p className="text-muted-foreground">{question.question}</p>
        <p>{question.answer ?? t.chat.question.notAnswered}</p>
      </div>
    );
  }
  return <Open turn={turn} question={question} />;
}

/** The card while it waits: the options as buttons, and a field. */
function Open({ turn, question }: { turn: number; question: QuestionView }) {
  const t = useT();
  const queryClient = useQueryClient();
  const [own, setOwn] = useState("");
  const answer = useMutation({
    mutationFn: (answer: string) => chatApi.answerQuestion(turn, answer),
    // The resumed turn arrives on the event stream too; re-reading the
    // conversation closes this card even if that event was missed.
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["chat"] }),
  });
  const typed = typedAnswer(own);
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-info/50 bg-info/5 p-3 text-sm">
      <p className="font-medium">{question.question}</p>
      <div className="flex flex-wrap gap-2">
        {question.options.map((option) => (
          <Button
            key={option}
            size="sm"
            variant="outline"
            disabled={answer.isPending}
            onClick={() => answer.mutate(option)}
          >
            {option}
          </Button>
        ))}
      </div>
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (typed) answer.mutate(typed);
        }}
      >
        <Input
          value={own}
          onChange={(event) => setOwn(event.target.value)}
          placeholder={t.chat.question.ownPlaceholder}
          disabled={answer.isPending}
          className="h-8 text-xs"
        />
        <Button type="submit" size="sm" disabled={answer.isPending || !typed}>
          {t.chat.question.answer}
        </Button>
      </form>
      {answer.isError && <p className="text-xs text-destructive">{answer.error.message}</p>}
    </div>
  );
}
