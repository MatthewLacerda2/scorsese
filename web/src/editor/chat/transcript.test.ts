import { describe, expect, test } from "bun:test";
import type { TurnView } from "@/api/chat";
import { ApiError } from "@/api/client";
import type { JobView, ServerEvent } from "@/api/events";
import { en } from "@/i18n/en";
import type { Entry, Transcript } from "./transcript";
import { apply, empty, generating, problem, running, upsert, words } from "./transcript";

function turn(id: number, fields: Partial<TurnView> = {}): TurnView {
  return {
    id,
    session: 1,
    project: 7,
    prompt: `prompt ${id}`,
    state: "running",
    answer: null,
    stop_reason: null,
    model: "claude-opus-5-5",
    calls: 0,
    input_tokens: 0,
    output_tokens: 0,
    cache_write_tokens: 0,
    cache_read_tokens: 0,
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    quote_replies: [],
    questions: [],
    started_at: 0,
    finished_at: null,
    ...fields,
  };
}

const started = upsert(empty(7), turn(1));

/** The transcript's first turn, which every test here is about. */
function first(transcript: Transcript): Entry {
  const [entry] = transcript.entries;
  if (!entry) throw new Error("the transcript has no turn");
  return entry;
}

describe("a running turn", () => {
  test("streams its words as they arrive, and never a thinking note or a tool call", () => {
    let shown = apply(started, { type: "chat_text", turn: 1, text: "Cutting " });
    shown = apply(shown, { type: "chat_text", turn: 1, text: "the intro." });
    // What a server from before #767 still sends while a deploy rolls over.
    const note = { type: "chat_progress", turn: 1, text: "Reading the project" };
    shown = apply(shown, note as unknown as ServerEvent);
    shown = apply(shown, {
      type: "chat_tool",
      turn: 1,
      tool: "clip_move",
      state: "running",
      said: null,
    });
    shown = apply(shown, {
      type: "chat_tool",
      turn: 1,
      tool: "clip_move",
      state: "answered",
      said: "c1 now…",
    });
    const entry = first(shown);
    // The server still announces its tool calls; the panel shows none of them (#1437).
    expect(words(entry)).toBe("Cutting the intro.");
    expect(Object.keys(entry).sort()).toEqual(["balanceAfter", "streamed", "turn"]);
    expect(running(shown)?.id).toBe(1);
  });

  test("ends with its stored answer, its cost and the balance after it", () => {
    const streamed = apply(started, { type: "chat_text", turn: 1, text: "half a thought" });
    const ended = turn(1, { state: "answered", answer: "Done.", charged_micros: 4_200 });
    const shown = apply(streamed, { type: "chat_turn", turn: ended, balance_micros: 995_800 });
    expect(words(first(shown))).toBe("Done.");
    expect(first(shown).balanceAfter).toBe(995_800);
    expect(running(shown)).toBeNull();
  });

  test("puts a quote it was sent on its turn, unanswered", () => {
    const quote = { tool: "generate", lines: [], micros: 1, expires_at: 9 };
    const shown = apply(started, { type: "chat_quote", turn: 1, quote });
    expect([first(shown).turn.quote, first(shown).turn.quote_answer]).toEqual([quote, null]);
  });
});

describe("what is not this panel's", () => {
  test("another project's turn and an unknown turn's words change nothing", () => {
    expect(upsert(started, turn(2, { project: 8 }))).toBe(started);
    expect(apply(started, { type: "chat_text", turn: 99, text: "x" })).toBe(started);
    expect(apply(started, { type: "project", id: 7, revision: 3 })).toBe(started);
  });

  test("only generation jobs are kept, each once, at its latest state", () => {
    const job = (id: number, kind: string, state: JobView["state"]): JobView => ({
      id,
      kind,
      state,
      attempts: 1,
      result: null,
      error: null,
      created_at: 0,
      started_at: null,
      finished_at: null,
      interrupted_at: null,
    });
    let shown = apply(started, { type: "job", ...job(5, "thumbnail", "done") });
    shown = apply(shown, { type: "job", ...job(6, "veo_shot", "waiting") });
    expect(generating(shown)).toBe(true);
    shown = apply(shown, { type: "job", ...job(6, "veo_shot", "running") });
    expect(shown.jobs.map((seen) => [seen.id, seen.state])).toEqual([[6, "running"]]);
    expect(generating(shown)).toBe(true);
    shown = apply(shown, { type: "job", ...job(6, "veo_shot", "done") });
    expect(generating(shown)).toBe(false);
  });
});

test("a server with no assistant key is a note, not an error", () => {
  expect(problem(new ApiError(503, "the assistant is not configured"), en.chat).tone).toBe("note");
  expect(problem(new ApiError(402, "no credit"), en.chat).lead).toContain("no credit");
  expect(problem(new Error("offline"), en.chat).detail).toBe("offline");
});
