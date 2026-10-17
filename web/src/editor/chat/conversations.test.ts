// Starting over and re-reading: how the panel's conversation is replaced (#1437).

import { describe, expect, test } from "bun:test";
import type { TurnView } from "@/api/chat";
import { apply, empty, fromConversation, restart, upsert } from "./transcript";

function turn(id: number, fields: Partial<TurnView> = {}): TurnView {
  return {
    ...({} as TurnView),
    id,
    session: 1,
    project: 7,
    prompt: `prompt ${id}`,
    state: "running",
    ...fields,
  };
}

const started = upsert(empty(7), turn(1));

describe("conversations", () => {
  test("a new conversation empties the panel and a re-read of the old one cannot refill it", () => {
    const conversation = {
      project: 7,
      session: 1,
      turns: [turn(1, { state: "answered" })],
      model: "m",
      models: [],
    };
    const old = fromConversation(conversation, empty(7));
    const cleared = restart(old);
    expect(cleared.entries).toEqual([]);
    expect(cleared.fresh).toBe(true);
    expect(fromConversation(conversation, cleared)).toBe(cleared);
    const begun = upsert(cleared, turn(2, { session: 2 }));
    expect(begun.fresh).toBe(false);
    expect(begun.entries.map((entry) => entry.turn.id)).toEqual([2]);
  });

  test("a re-read keeps what was seen live of each turn", () => {
    const live = apply(started, { type: "chat_text", turn: 1, text: "Cutting" });
    const reread = fromConversation(
      {
        project: 7,
        session: 1,
        turns: [turn(1, { state: "answered" })],
        model: "claude-sonnet-5-5",
        models: [],
      },
      live,
    );
    expect(first(reread).streamed).toBe("Cutting");
    expect(first(reread).turn.state).toBe("answered");
  });

  test("a turn of a newer conversation starts the panel over", () => {
    const fresh = upsert(started, turn(2, { session: 2 }));
    expect(fresh.entries.map((entry) => entry.turn.id)).toEqual([2]);
    expect(upsert(fresh, turn(3, { session: 1 }))).toBe(fresh);
  });
});
