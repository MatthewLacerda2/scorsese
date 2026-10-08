// The picker's modal (#901): a grid of the candidates, each enlarged — a video
// played with its sound off — on a click; one or more selected with their
// check; "none of these"; and a field for the user's own words. Everything it
// shows is the source's own preview and file, credited to it; nothing is
// downloaded until the user has picked, and then only what they picked. A
// Lottie animation (#908) is a picture that moves — its GIF, grid and enlarged
// alike — with LottieFiles named on its tile, since the rest come from Pixabay.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CheckIcon, ChevronLeftIcon } from "lucide-react";
import { useState } from "react";
import { type CandidateView, chatApi, type QuestionView } from "@/api/chat";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useT } from "@/i18n/I18nProvider";
import { length, nameOf, sourceName, toggle } from "./picker";

export function PickerModal({
  turn,
  question,
  open,
  onOpenChange,
}: {
  turn: number;
  question: QuestionView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const candidates = question.candidates ?? [];
  const [selected, setSelected] = useState<string[]>([]);
  const [looking, setLooking] = useState<CandidateView | null>(null);
  const [own, setOwn] = useState("");
  const answer = useMutation({
    mutationFn: (picked: string[]) => chatApi.answerQuestion(turn, own.trim(), picked),
    // The resumed turn arrives on the event stream too; re-reading the
    // conversation closes the picker even if that event was missed.
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["chat"] }),
  });
  const choose = (key: string) => setSelected((now) => toggle(now, key, candidates));
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{question.question}</DialogTitle>
          <DialogDescription>{t.chat.picker.hint}</DialogDescription>
        </DialogHeader>
        {looking ? (
          <Enlarged
            candidate={looking}
            selected={selected.includes(looking.key)}
            onBack={() => setLooking(null)}
            onChoose={() => choose(looking.key)}
          />
        ) : (
          <div className="grid max-h-[60vh] grid-cols-2 gap-2 overflow-y-auto sm:grid-cols-3">
            {candidates.map((one) => (
              <Tile
                key={one.key}
                candidate={one}
                selected={selected.includes(one.key)}
                onLook={() => setLooking(one)}
                onChoose={() => choose(one.key)}
              />
            ))}
          </div>
        )}
        <Input
          value={own}
          onChange={(event) => setOwn(event.target.value)}
          placeholder={t.chat.picker.ownPlaceholder}
          disabled={answer.isPending}
          className="h-8 text-xs"
        />
        {answer.isError && <p className="text-xs text-destructive">{answer.error.message}</p>}
        <DialogFooter className="items-center sm:justify-between">
          <p className="text-xs text-muted-foreground">
            {t.chat.picker.from(sourceName(candidates))}
          </p>
          <div className="flex gap-2">
            <Button variant="outline" disabled={answer.isPending} onClick={() => answer.mutate([])}>
              {own.trim() ? t.chat.picker.noneSay : t.chat.picker.none}
            </Button>
            <Button
              disabled={answer.isPending || selected.length === 0}
              onClick={() => answer.mutate(selected)}
            >
              {t.chat.picker.use(selected.length)}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** One candidate in the grid: its picture to enlarge, its check to select. */
function Tile({
  candidate,
  selected,
  onLook,
  onChoose,
}: {
  candidate: CandidateView;
  selected: boolean;
  onLook: () => void;
  onChoose: () => void;
}) {
  const t = useT();
  const long = length(candidate.seconds);
  return (
    <div
      className={`relative overflow-hidden rounded-md ring-2 ${selected ? "ring-sky-500" : "ring-transparent"}`}
    >
      <button
        type="button"
        onClick={onLook}
        aria-label={t.chat.picker.enlarge}
        className="block w-full"
      >
        <img
          src={candidate.preview_url}
          alt={candidate.tags.join(", ")}
          loading="lazy"
          className="aspect-video w-full bg-muted object-cover"
        />
      </button>
      {long && (
        <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 text-[10px] text-white">
          {long}
        </span>
      )}
      {candidate.kind === "lottie" && (
        <span className="absolute top-1 left-1 rounded bg-black/70 px-1 text-[10px] text-white">
          {nameOf(candidate.source)}
        </span>
      )}
      <button
        type="button"
        onClick={onChoose}
        aria-pressed={selected}
        aria-label={selected ? t.chat.picker.selected : t.chat.picker.select}
        className={`absolute top-1 right-1 flex size-6 items-center justify-center rounded-full border-2 border-white shadow ${selected ? "bg-sky-500 text-white" : "bg-black/30"}`}
      >
        {selected && <CheckIcon className="size-4" />}
      </button>
    </div>
  );
}

/** One candidate, large: a video plays muted, a picture — or a Lottie's GIF — shows whole. */
function Enlarged({
  candidate,
  selected,
  onBack,
  onChoose,
}: {
  candidate: CandidateView;
  selected: boolean;
  onBack: () => void;
  onChoose: () => void;
}) {
  const t = useT();
  return (
    <div className="flex flex-col gap-2">
      {candidate.kind === "video" ? (
        <video
          src={candidate.look_url}
          poster={candidate.preview_url}
          autoPlay
          muted
          loop
          playsInline
          controls
          className="max-h-[55vh] w-full rounded-md bg-black object-contain"
        />
      ) : (
        <img
          src={candidate.look_url}
          alt={candidate.tags.join(", ")}
          className="max-h-[55vh] w-full rounded-md bg-muted object-contain"
        />
      )}
      <div className="flex items-center justify-between gap-2 text-xs">
        <Button size="sm" variant="ghost" onClick={onBack}>
          <ChevronLeftIcon /> {t.chat.picker.back}
        </Button>
        <a
          href={candidate.page_url}
          target="_blank"
          rel="noreferrer"
          className="truncate text-muted-foreground underline"
        >
          {t.chat.picker.by(candidate.author)}
        </a>
        <Button size="sm" variant={selected ? "default" : "outline"} onClick={onChoose}>
          {selected ? t.chat.picker.selected : t.chat.picker.select}
        </Button>
      </div>
    </div>
  );
}
