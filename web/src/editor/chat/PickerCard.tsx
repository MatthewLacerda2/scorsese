// The assistant's stock picker (#901): a question with pictures for options.
// While it waits, the card in the chat opens a modal (`PickerModal.tsx`) — on
// its own the first time, and again from the card's button. Once answered,
// the card shrinks to the question and what was picked. A message written in
// the composer instead answers it too, as it does a question.

import { useState } from "react";
import type { CandidateView, QuestionView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/I18nProvider";
import { PickerModal } from "./PickerModal";
import { pickedOf } from "./picker";

export function PickerCard({
  turn,
  question,
  waiting,
}: {
  turn: number;
  question: QuestionView;
  /** Whether this is the picker the turn is paused on now. */
  waiting: boolean;
}) {
  if (!waiting) return <Answered question={question} />;
  return <Open turn={turn} question={question} />;
}

/** The card while it waits, and the modal it opens. */
function Open({ turn, question }: { turn: number; question: QuestionView }) {
  const t = useT();
  const [open, setOpen] = useState(true);
  const candidates = question.candidates ?? [];
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-sky-500/50 bg-sky-500/5 p-3 text-sm">
      <p className="font-medium">{question.question}</p>
      <Strip candidates={candidates.slice(0, 4)} />
      <Button size="sm" className="self-start" onClick={() => setOpen(true)}>
        {t.chat.picker.see(candidates.length)}
      </Button>
      <PickerModal turn={turn} question={question} open={open} onOpenChange={setOpen} />
    </div>
  );
}

/** An answered picker: the question, and the pictures picked or the words written. */
function Answered({ question }: { question: QuestionView }) {
  const t = useT();
  const picked = pickedOf(question);
  return (
    <div className="flex flex-col gap-1 rounded-lg border px-3 py-2 text-xs">
      <p className="text-muted-foreground">{question.question}</p>
      {picked.length > 0 && <Strip candidates={picked} />}
      {question.picked !== undefined && picked.length === 0 && <p>{t.chat.picker.noneTaken}</p>}
      {question.answer !== null && <p>{question.answer}</p>}
      {question.picked === undefined && question.answer === null && (
        <p>{t.chat.question.notAnswered}</p>
      )}
    </div>
  );
}

/** A row of small previews. */
function Strip({ candidates }: { candidates: CandidateView[] }) {
  return (
    <div className="flex gap-1.5 overflow-hidden">
      {candidates.map((one) => (
        <img
          key={one.key}
          src={one.preview_url}
          alt={one.tags.join(", ")}
          loading="lazy"
          className="h-12 w-20 shrink-0 rounded object-cover"
        />
      ))}
    </div>
  );
}
