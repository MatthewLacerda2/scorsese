import { describe, expect, test } from "bun:test";
import type { ModelChoice, TurnView } from "@/api/chat";
import { warnsBeforeSwitching } from "./switching";

const sonnet: ModelChoice = {
  id: "claude-sonnet-5-5",
  label: "Claude Sonnet 5.5",
  vendor: "anthropic",
  unavailable: null,
  cache_seconds: 3600,
  cost: 50,
};

function turn(finished_at: number | null): TurnView {
  return {
    id: 1,
    session: 1,
    project: 1,
    prompt: "make an intro",
    state: finished_at === null ? "running" : "answered",
    answer: null,
    stop_reason: null,
    model: sonnet.id,
    calls: 1,
    input_tokens: 0,
    output_tokens: 0,
    cache_write_tokens: 0,
    cache_read_tokens: 0,
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    quote_replies: [],
    woken: false,
    questions: [],
    started_at: 0,
    finished_at,
  };
}

describe("the switch warning", () => {
  test("an empty conversation has nothing cached", () => {
    expect(warnsBeforeSwitching([], sonnet, 10_000)).toBe(false);
  });

  test("an answer within the cache's hour warns", () => {
    expect(warnsBeforeSwitching([turn(10_000)], sonnet, 10_000 + 3_599)).toBe(true);
    expect(warnsBeforeSwitching([turn(10_000)], sonnet, 10_000 + 3_600)).toBe(true);
  });

  test("an answer older than the cache's lifetime does not", () => {
    expect(warnsBeforeSwitching([turn(10_000)], sonnet, 10_000 + 3_601)).toBe(false);
  });

  test("the lifetime is the model being left's", () => {
    const brief = { ...sonnet, cache_seconds: 300 };
    expect(warnsBeforeSwitching([turn(10_000)], brief, 10_000 + 301)).toBe(false);
  });

  test("a turn still running warns", () => {
    expect(warnsBeforeSwitching([turn(10_000), turn(null)], sonnet, 99_999)).toBe(true);
  });
});
