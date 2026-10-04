// The assistant's routes (docs/web.md, *Assistant turns*; `http/chat.rs`),
// and the shapes they answer with, named after the Rust types they mirror in
// `crates/server/src/assistant/`. A turn streams on `GET /api/events`; these
// start one, read one back, stop one, and answer its quote.

import { request } from "./client";

/** How a turn ended, or `running` while it goes. */
export type TurnState =
  | "running"
  | "answered"
  | "refused"
  | "capped"
  | "stopped"
  | "failed"
  | "interrupted";

/** `assistant::QuoteView` — what a paid tool would cost, waiting for a yes. */
export interface QuoteView {
  tool: string;
  /** Each brief's cost and the total, in words. */
  lines: string[];
  /** What it takes from the balance, in micro-dollars. */
  micros: number;
  expires_at: number;
}

/** `assistant::TurnView` — one prompt and what the assistant did with it. */
export interface TurnView {
  id: number;
  session: number;
  project: number;
  prompt: string;
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
  /** `null` while the confirmation box should show. */
  quote_answer: "confirmed" | "declined" | "withdrawn" | null;
  started_at: number;
  finished_at: number | null;
}

/** `assistant::Choice` — one model the assistant can run on (#705). */
export interface ModelChoice {
  /** What the project stores, e.g. `gemini-3.8-flash`. */
  id: string;
  label: string;
  vendor: "anthropic" | "google";
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
}

/** `assistant::ToolCallView` — one tool call a turn made. */
export interface ToolCallView {
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

/** `assistant::Answered` — what answering a quote did. */
export interface Answered {
  /** What the paid call said, after a yes. */
  spent: string | null;
  refused: boolean;
  /** The turn carrying on after a yes. */
  turn: TurnView | null;
  note: string | null;
}

export const chatApi = {
  conversation: (projectId: number) => request<Conversation>("GET", `/projects/${projectId}/chat`),
  /** `202` with the turn; `402` no credit, `409` one is running, `503` not configured. */
  send: (projectId: number, prompt: string, fresh = false) =>
    request<TurnView>("POST", `/projects/${projectId}/chat`, { prompt, fresh }),
  /** The project's assistant runs on `model` from its next turn; `400` for one not offered. */
  chooseModel: (projectId: number, model: string) =>
    request<ModelChoice>("PUT", `/projects/${projectId}/chat/model`, { model }),
  turn: (turnId: number) => request<TurnDetail>("GET", `/chat/turns/${turnId}`),
  stop: (turnId: number) => request<{ stopping: number }>("POST", `/chat/turns/${turnId}/stop`),
  answerQuote: (turnId: number, confirm: boolean) =>
    request<Answered>("POST", `/chat/turns/${turnId}/quote`, { confirm }),
};
