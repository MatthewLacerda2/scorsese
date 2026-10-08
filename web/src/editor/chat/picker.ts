// What the stock picker needs worked out before it draws (#901): which
// candidates are selected, in the order they were offered, and what an answer
// sends. Kept apart from React so it is tested as plain data.

import type { CandidateView, QuestionView } from "@/api/chat";

/** Whether `question` is a picker rather than a question in words. */
export function isPicker(question: QuestionView): boolean {
  return (question.candidates?.length ?? 0) > 0;
}

/** `selected` with `key` added or taken away, kept in the order `candidates` offer them. */
export function toggle(selected: string[], key: string, candidates: CandidateView[]): string[] {
  const chosen = new Set(selected);
  if (chosen.has(key)) chosen.delete(key);
  else chosen.add(key);
  return candidates.map((one) => one.key).filter((one) => chosen.has(one));
}

/** The candidates an answered picker's user picked, in the order shown. */
export function pickedOf(question: QuestionView): CandidateView[] {
  const picked = new Set(question.picked ?? []);
  return (question.candidates ?? []).filter((one) => picked.has(one.key));
}

/** A clip's length as a badge reads it: `0:08`, `1:30`. */
export function length(seconds: number | null): string | null {
  if (seconds === null || seconds < 0) return null;
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(Math.round(seconds % 60)).padStart(2, "0")}`;
}

/** The source as the picker credits it, which Pixabay asks of any displayed results. */
export function sourceName(candidates: CandidateView[]): string {
  const names: Record<string, string> = { pixabay: "Pixabay" };
  const sources = [...new Set(candidates.map((one) => one.source))];
  return sources.map((source) => names[source] ?? source).join(", ");
}
