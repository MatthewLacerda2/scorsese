// What `GET /api/events` sends: one server-sent stream per user, each message
// a JSON object tagged by `type` — `crates/server/src/events.rs`'s `Event`.
// In memory on the server and allowed to drop, so `resync` (or reconnecting)
// means "re-read what you show"; the database is the record, this the nudge.

import type { QuoteView, TurnView } from "./chat";

/** Where a job is in its life — `jobs::State`. */
export type JobState = "waiting" | "running" | "done" | "failed" | "stuck" | "cancelled";

/** `jobs::ProgressView` — how far a running render or preview has got (#698). */
export interface JobProgress {
  /** 0–100, frames only: 0 while preparing or mixing, 99 while finishing, 100 once the file is there. */
  percent: number;
  phase: "preparing" | "mixing" | "drawing" | "finishing" | "done";
  /** Frames encoded so far, of `of`; `of` is 0 until planned and for a sound-only render. */
  done: number;
  of: number;
}

/** `jobs::JobView` — a job as its owner sees it. Times are Unix seconds. */
export interface JobView {
  id: number;
  /** `render`, `thumbnail`, `veo_shot`, `still_image`, `spoken_line`, … */
  kind: string;
  state: JobState;
  attempts: number;
  /** What it produced, once done; a render's names its file. */
  result: Record<string, unknown> | null;
  /** Why it failed or is stuck, or how far it got before it was cancelled. */
  error: string | null;
  created_at: number;
  started_at: number | null;
  finished_at: number | null;
  interrupted_at: number | null;
  /** How far it has got, while a render or preview runs; otherwise null (or absent). */
  progress?: JobProgress | null;
}

/** Every message on the stream. */
export type ServerEvent =
  | ({ type: "job" } & JobView)
  /** A running job of theirs got further: at most twice a second. */
  | { type: "job_progress"; id: number; progress: JobProgress }
  /** One of their projects changed: re-read it if it is on screen. */
  | { type: "project"; id: number; revision: number }
  /** A turn started, was charged for a call, or ended, and the balance after. */
  | { type: "chat_turn"; turn: TurnView; balance_micros: number }
  /** More of the words the assistant is writing: append them. */
  | { type: "chat_text"; turn: number; text: string }
  /** A tool `running`, then `answered` or `refused`, with the start of its answer. */
  | {
      type: "chat_tool";
      turn: number;
      tool: string;
      state: "running" | "answered" | "refused";
      said: string | null;
    }
  /** A paid tool's quote, waiting for the user's yes or no. */
  | { type: "chat_quote"; turn: number; quote: QuoteView }
  /** Messages were dropped (or the stream reconnected): re-read what is shown. */
  | { type: "resync" };
