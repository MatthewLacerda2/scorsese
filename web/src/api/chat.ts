// The assistant's routes (docs/web.md, *Assistant turns*; `http/chat.rs`),
// and the shapes they answer with, named after the Rust types they mirror in
// `crates/server/src/assistant/`. A turn streams on `GET /api/events`; these
// start one, read one back, stop one, and answer its quote or its question.

import { request } from "./client";

/** How a turn ended, `running` while it goes, or `asking` while it waits on a question. */
export type TurnState =
  | "running"
  | "asking"
  | "answered"
  | "refused"
  | "capped"
  | "stopped"
  | "failed"
  | "interrupted";

/** `assistant::QuoteItem` — one brief a yes pays for, and what it would send. */
export interface QuoteItem {
  /** An asset id, or `design` for a voice design. */
  subject: string;
  /** The price, and how it got there. */
  says: string;
  /** What it takes from the balance, in micro-dollars: a quote's items add up to its `micros`. */
  micros: number;
  /** What `description` is. */
  brief: "prompt" | "line" | "voice";
  description: string;
}

/** `assistant::QuoteView` — what a paid tool would cost, waiting for a yes. */
export interface QuoteView {
  tool: string;
  /** Each item with its description (#709); absent on a quote held before. */
  items?: QuoteItem[];
  /** The total, and any line no item claims, in words. */
  lines: string[];
  /** What it takes from the balance, in micro-dollars. */
  micros: number;
  expires_at: number;
  /** The call quoted its stills as a half-price batch (#947). */
  batched?: boolean;
  /**
   * What the half-price batch offered beside a quote for now would take
   * instead, in micro-dollars (#947) — the box's second yes. Absent or `null`
   * when nothing was offered.
   */
  batch_micros?: number | null;
}

/** `assistant::QuoteReply` — one answer a quote was given (#1440). */
export interface QuoteReply {
  /** `changed` is a change asked for instead (#709); `withdrawn`, left unanswered. */
  answer: "confirmed" | "declined" | "changed" | "withdrawn";
  /** What the quote would have taken, in micro-dollars. */
  micros: number;
  /** The person's own words: the change asked for, or the message written instead. */
  words: string | null;
}

/**
 * `assistant::QuestionView` — a question the assistant asked mid-turn (#710),
 * or a picker (#901): the same question with stock candidates to pick from.
 */
export interface QuestionView {
  question: string;
  /** Two to four answers to pick from; the user may always write their own. Empty on a picker. */
  options: string[];
  /** What they answered — on a picker, the words beside their pick — or `null`. */
  answer: string | null;
  /** A picker's candidates, in the order offered; absent on a question in words. */
  candidates?: CandidateView[];
  /** The keys picked, in the order shown (`[]` is none of them); absent while it waits or when answered in words. */
  picked?: string[];
}

/**
 * `assistant::CandidateView` — one result a picker offers (#901). `source` and
 * `kind` say what it is (`pixabay` with `video` or `image`, `lottiefiles` with
 * `lottie`, #908); every URL is the source's own, shown while the user chooses
 * and never kept.
 */
export interface CandidateView {
  /** What a pick names it by. */
  key: string;
  source: string;
  kind: string;
  id: string;
  /** A still for the grid — a Lottie's animated GIF. */
  preview_url: string;
  /** What the enlarged view shows: a video's smallest file (played muted), or a picture — a Lottie's GIF. */
  look_url: string;
  width: number;
  height: number;
  seconds: number | null;
  author: string;
  page_url: string;
  tags: string[];
}

/** `assistant::TurnView` — one prompt and what the assistant did with it. */
export interface TurnView {
  id: number;
  session: number;
  project: number;
  prompt: string;
  /** Started by the server when the work the conversation ordered landed (#1441), not by a message: shown as a line, never a prompt bubble. */
  woken: boolean;
  state: TurnState;
  /** The final answer, or why there is none. */
  answer: string | null;
  stop_reason: string | null;
  model: string;
  calls: number;
  input_tokens: number;
  output_tokens: number;
  cache_write_tokens: number;
  cache_read_tokens: number;
  /** What it has cost so far, in micro-dollars. */
  charged_micros: number;
  quote: QuoteView | null;
  /** How the latest quote was answered; `null` while it waits. */
  quote_answer: "confirmed" | "declined" | "withdrawn" | null;
  /** Every answer its quotes were given, in order: one line each once answered (#1440). */
  quote_replies: QuoteReply[];
  /** Every question it asked, in order; the last waits while the turn is `asking`. */
  questions: QuestionView[];
  started_at: number;
  finished_at: number | null;
}

/** `assistant::Choice` — one model the assistant can run on (#705). */
export interface ModelChoice {
  /** What the project stores, e.g. `claude-sonnet-5-5`. */
  id: string;
  label: string;
  /** Anthropic's, the only vendor offered since #1220. */
  vendor: "anthropic";
  /** Why a turn on it would be refused right now, in the server's words; `null` when it can answer. */
  unavailable: string | null;
  /** How long after the last answer a switch away from it can still miss a warm cache. */
  cache_seconds: number;
  /** How dear it is beside the others, as a percentage of the dearest (1–100): the
   * picker's cost bar. `null` when the server has no price for it. */
  cost: number | null;
}

/** `assistant::Conversation` — the project's newest conversation, oldest turn first. */
export interface Conversation {
  project: number;
  /** `null` before the first turn. */
  session: number | null;
  turns: TurnView[];
  /** The id of the model the project's next turn runs on. */
  model: string;
  /** Every model the picker offers, in order. */
  models: ModelChoice[];
  /** How hard the project's next turn thinks (#1443): `low`, `medium`, `high` or `xhigh`. */
  effort: string;
}

/** `assistant::ToolCallView` — one tool call a turn made. */
interface ToolCallView {
  id: number;
  position: number;
  client: string;
  tool: string;
  arguments: Record<string, unknown>;
  outcome: "answered" | "refused" | null;
  reply: string | null;
  started_at: number;
  finished_at: number | null;
}

/** `assistant::TurnDetail` — a turn and every tool call it made. */
export interface TurnDetail {
  turn: TurnView;
  tools: ToolCallView[];
}

/**
 * `http::chat::QuoteAnswer` — yes (to the batch offered beside the quote, with
 * `batch`), no, or a change asked for (`confirm: false`).
 */
export type QuoteAnswer = { confirm: true; batch?: true } | { confirm: false; change?: string };

/** `assistant::Answered` — what answering a quote did. */
export interface Answered {
  /** What the paid call said, after a yes. */
  spent: string | null;
  refused: boolean;
  /** The turn the quote paused, carrying on with the answer (#1440). */
  turn: TurnView | null;
  note: string | null;
}

export const chatApi = {
  conversation: (projectId: number) => request<Conversation>("GET", `/projects/${projectId}/chat`),
  /**
   * `202` with the turn, at the project's effort; `402` no credit, `409` one is
   * running, `503` not configured. `attachments` are library ids (#1534): the
   * server brings the media into the project and names them to the model.
   */
  send: (projectId: number, prompt: string, fresh = false, attachments: number[] = []) =>
    request<TurnView>("POST", `/projects/${projectId}/chat`, { prompt, fresh, attachments }),
  /** The project's assistant runs on `model` from its next turn; `400` for one not offered. */
  chooseModel: (projectId: number, model: string) =>
    request<ModelChoice>("PUT", `/projects/${projectId}/chat/model`, { model }),
  /** The project's assistant runs at `effort` from its next turn; `400` for one not offered. */
  chooseEffort: (projectId: number, effort: string) =>
    request<{ effort: string }>("PUT", `/projects/${projectId}/chat/effort`, { effort }),
  turn: (turnId: number) => request<TurnDetail>("GET", `/chat/turns/${turnId}`),
  stop: (turnId: number) => request<{ stopping: number }>("POST", `/chat/turns/${turnId}/stop`),
  /** Spend it, withdraw it, or withdraw it asking for `change` — one call, nothing spent. */
  answerQuote: (turnId: number, answer: QuoteAnswer) =>
    request<Answered>("POST", `/chat/turns/${turnId}/quote`, answer),
  /**
   * Answer the question the turn waits on — or, with `picked`, its picker's
   * candidates (`[]` is none): `202` with the same turn, running again.
   */
  answerQuestion: (turnId: number, answer: string, picked?: string[]) =>
    request<TurnView>("POST", `/chat/turns/${turnId}/answer`, { answer, picked }),
};
