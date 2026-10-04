import { describe, expect, test } from "bun:test";
import type { QuestionView, TurnView } from "@/api/chat";
import { typedAnswer } from "./question";
import { asking, empty, upsert, waitingQuestion } from "./transcript";

const asked: QuestionView = { question: "Loop or fade?", options: ["loop", "fade"], answer: null };

function turn(fields: Partial<TurnView>): TurnView {
  return {
    id: 3,
    session: 1,
    project: 7,
    prompt: "add the music",
    state: "asking",
    answer: null,
    stop_reason: null,
    model: "gemini-3.8-flash",
    calls: 1,
    input_tokens: 0,
    output_tokens: 0,
    cache_write_tokens: 0,
    cache_read_tokens: 0,
    charged_micros: 2_888,
    quote: null,
    quote_answer: null,
    questions: [asked],
    started_at: 0,
    finished_at: null,
    ...fields,
  };
}

describe("waitingQuestion", () => {
  test("is the last question of a turn that is asking", () => {
    expect(waitingQuestion(turn({}))).toEqual(asked);
  });

  test("is nothing once it is answered and the turn runs again", () => {
    const answered = { ...asked, answer: "fade" };
    expect(waitingQuestion(turn({ state: "running", questions: [answered] }))).toBeNull();
  });

  test("is nothing for a turn set aside with its question unanswered", () => {
    expect(waitingQuestion(turn({ state: "stopped" }))).toBeNull();
  });

  test("tolerates a turn read from before questions existed", () => {
    const old = turn({}) as Partial<TurnView>;
    delete old.questions;
    expect(waitingQuestion(old as TurnView)).toBeNull();
  });
});

describe("asking", () => {
  test("finds the turn paused on a question, which no longer holds Send", () => {
    const shown = upsert(empty(7), turn({}));
    expect(asking(shown)?.id).toBe(3);
    expect(asking(upsert(shown, turn({ state: "running" })))).toBeNull();
  });
});

describe("typedAnswer", () => {
  test("sends the words, trimmed", () => {
    expect(typedAnswer("  loop it, quieter ")).toBe("loop it, quieter");
  });

  test("sends nothing for an empty field", () => {
    expect(typedAnswer("  ")).toBeNull();
  });
});
