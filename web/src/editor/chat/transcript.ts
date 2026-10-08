// What the chat panel shows, as plain data and pure functions: the stored
// conversation (`GET /api/projects/{id}/chat`) folded together with the live
// events that arrive while a turn runs. Kept apart from React so the folding —
// which is where a lost line or a doubled one would come from — is tested as
// arithmetic rather than by clicking.
//
// What is stored and what is live differ on purpose. The server keeps a turn's
// prompt, final answer, state and cost; the words as they stream and the tool
// lines exist only on the event stream. The model's thinking is on neither
// (#767): a working turn shows a status word instead (`status.ts`). So a re-read after a
// `resync` replaces every turn but keeps the live lines already seen for it.

import type { Conversation, QuestionView, TurnState, TurnView } from "@/api/chat";
import { ApiError } from "@/api/client";
import type { JobView, ServerEvent } from "@/api/events";
import type { Messages } from "@/i18n/catalogue";

/** Where a tool the assistant called is. */
export type ToolState = "running" | "answered" | "refused";

/** One line between a prompt and its answer: a tool the assistant called. */
export type Line = { kind: "tool"; tool: string; state: ToolState; said: string | null };

/** A turn as the panel shows it. */
export interface Entry {
  turn: TurnView;
  /** The words streamed so far (`chat_text`), before the answer is stored. */
  streamed: string;
  lines: Line[];
  /** The balance after the turn's last charge, when an event said it. */
  balanceAfter: number | null;
}

/** The whole panel. */
export interface Transcript {
  project: number;
  session: number | null;
  entries: Entry[];
  /** Generation jobs seen since the panel opened, newest last. */
  jobs: JobView[];
}

/** The kinds of job that are the assistant's generations, shown in the panel. */
const GENERATIONS = new Set(["veo_shot", "still_image", "spoken_line", "voice_design"]);

export function empty(project: number): Transcript {
  return { project, session: null, entries: [], jobs: [] };
}

/** The stored conversation, keeping what `previous` saw live of each turn. */
export function fromConversation(conversation: Conversation, previous: Transcript): Transcript {
  const seen = new Map(previous.entries.map((entry) => [entry.turn.id, entry]));
  return {
    ...previous,
    project: conversation.project,
    session: conversation.session,
    entries: conversation.turns.map((turn) => {
      const known = seen.get(turn.id);
      return known ? { ...known, turn } : { turn, streamed: "", lines: [], balanceAfter: null };
    }),
  };
}

/**
 * `turn` as it now stands: replaced where it is known, appended where it is
 * new. A turn of a newer conversation — somebody started a fresh one — starts
 * the transcript over, because the panel shows the newest conversation only.
 */
export function upsert(transcript: Transcript, turn: TurnView, balance?: number): Transcript {
  if (turn.project !== transcript.project) return transcript;
  const balanceAfter = (entry: Entry) => (balance === undefined ? entry.balanceAfter : balance);
  if (transcript.entries.some((entry) => entry.turn.id === turn.id)) {
    return change(transcript, turn.id, (entry) => ({
      ...entry,
      turn,
      balanceAfter: balanceAfter(entry),
    }));
  }
  const fresh: Entry = { turn, streamed: "", lines: [], balanceAfter: balance ?? null };
  if (transcript.session !== null && turn.session > transcript.session) {
    return { ...transcript, session: turn.session, entries: [fresh] };
  }
  if (transcript.session !== null && turn.session < transcript.session) return transcript;
  return { ...transcript, session: turn.session, entries: [...transcript.entries, fresh] };
}

/** The transcript with `event` folded in; unchanged by anything not its own. */
export function apply(transcript: Transcript, event: ServerEvent): Transcript {
  switch (event.type) {
    case "chat_turn":
      return upsert(transcript, event.turn, event.balance_micros);
    case "chat_text":
      return change(transcript, event.turn, (entry) => ({
        ...entry,
        streamed: entry.streamed + event.text,
      }));
    case "chat_tool":
      return change(transcript, event.turn, (entry) => ({
        ...entry,
        lines: toolLine(entry.lines, event.tool, event.state, event.said),
      }));
    case "chat_quote":
      return change(transcript, event.turn, (entry) => ({
        ...entry,
        turn: { ...entry.turn, quote: event.quote, quote_answer: null },
      }));
    case "job": {
      if (!GENERATIONS.has(event.kind)) return transcript;
      const { type: _, ...job } = event;
      const known = transcript.jobs.some((seen) => seen.id === job.id);
      const jobs = known
        ? transcript.jobs.map((seen) => (seen.id === job.id ? job : seen))
        : [...transcript.jobs, job];
      return { ...transcript, jobs };
    }
    default:
      return transcript;
  }
}

/** A tool's answer lands on the line that said it was running; a new call is a new line. */
function toolLine(lines: Line[], tool: string, state: ToolState, said: string | null): Line[] {
  const line: Line = { kind: "tool", tool, state, said };
  if (state !== "running") {
    const at = lines.findLastIndex(
      (known) => known.kind === "tool" && known.tool === tool && known.state === "running",
    );
    if (at >= 0) return lines.map((known, index) => (index === at ? line : known));
  }
  return [...lines, line];
}

function change(transcript: Transcript, turn: number, edit: (entry: Entry) => Entry): Transcript {
  if (!transcript.entries.some((entry) => entry.turn.id === turn)) return transcript;
  return {
    ...transcript,
    entries: transcript.entries.map((entry) => (entry.turn.id === turn ? edit(entry) : entry)),
  };
}

/** The turn still running, if one is: what Stop stops and what holds Send. */
export function running(transcript: Transcript): TurnView | null {
  return transcript.entries.find((entry) => entry.turn.state === "running")?.turn ?? null;
}

/**
 * The turn paused on a question, if one is (#710): Stop sets the question
 * aside, and Send stays open, because a message written then is its answer.
 */
export function asking(transcript: Transcript): TurnView | null {
  return transcript.entries.find((entry) => entry.turn.state === "asking")?.turn ?? null;
}

/** The question `turn` waits on, or `null`. */
export function waitingQuestion(turn: TurnView): QuestionView | null {
  if (turn.state !== "asking") return null;
  const last = (turn.questions ?? []).at(-1);
  return last && last.answer === null && last.picked === undefined ? last : null;
}

/** True while a turn's quote waits for the user's yes or no. */
export function awaitingQuote(turn: TurnView): boolean {
  return turn.quote !== null && turn.quote_answer === null;
}

/** What the assistant said: its stored answer once it ended, the stream until then. */
export function words(entry: Entry): string {
  if (entry.turn.state !== "running" && entry.turn.answer) return entry.turn.answer;
  return entry.streamed;
}

/** A turn that ended without answering, said plainly; `null` for one that answered or runs. */
export function ending(state: TurnState, t: Messages["chat"]): string | null {
  switch (state) {
    case "refused":
    case "capped":
    case "stopped":
    case "failed":
    case "interrupted":
      return t.ending[state];
    default:
      return null;
  }
}

/** A generation job's state, in the panel's words. */
export function jobState(job: JobView, t: Messages["chat"]): string {
  switch (job.state) {
    case "failed":
    case "stuck":
      return t.jobState[job.state](job.error);
    default:
      return t.jobState[job.state];
  }
}

/** A refused request, as the panel shows it: a lead line and the server's own words. */
export interface Problem {
  /** `note` for "not set up yet", which is not the user's doing; `error` otherwise. */
  tone: "note" | "error";
  lead: string | null;
  detail: string;
}

export function problem(error: unknown, t: Messages["chat"]): Problem {
  const detail = error instanceof Error ? error.message : String(error);
  if (!(error instanceof ApiError)) return { tone: "error", lead: null, detail };
  switch (error.status) {
    case 503:
      return { tone: "note", lead: t.problem.unconfigured, detail };
    case 402:
      return { tone: "error", lead: t.problem.noCredit, detail };
    case 409:
      return { tone: "error", lead: t.problem.busy, detail };
    default:
      return { tone: "error", lead: null, detail };
  }
}
