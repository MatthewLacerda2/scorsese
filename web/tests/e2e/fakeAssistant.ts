// The assistant, faked in the browser, for the flows that run a turn: every
// route a turn lives on answers from here, so no request that would spend
// money leaves the page, and a turn stays `running` for as long as a flow
// wants to look at it (#1429). What the server alone decides — the project
// change a new platform makes — still goes to the real server first.
//
// The live stream is faked too: `GET /api/events` answers whatever turns
// changed since the last connection and then closes, asking for a reconnect a
// quarter of a second later. The page reads each reconnect as a `resync` and
// re-reads the conversation, which answers from here as well — so a change
// reaches the panel by both of the ways a real one can.

import type { Page } from "@playwright/test";
import type { Answered, QuoteView, TurnView } from "@/api/chat";

/** The session every faked turn belongs to. */
const SESSION = 1;

export interface FakeAssistant {
  /** Ends `turn`, answered with `answer` (and, with `quote`, waiting on it). */
  finish(turn: TurnView, answer: string, quote?: QuoteView): void;
  /** The turn the last request started. */
  latest(): TurnView;
}

/** Fake the assistant on `page`. Call before the page opens, so the stream is faked from its first connection. */
export async function fakeAssistant(page: Page): Promise<FakeAssistant> {
  const turns: TurnView[] = [];
  let unsent: TurnView[] = [];
  const keep = (turn: TurnView) => {
    const at = turns.findIndex((known) => known.id === turn.id);
    if (at >= 0) turns[at] = turn;
    else turns.push(turn);
    unsent.push(turn);
    return turn;
  };
  const begin = (project: number, prompt: string) =>
    keep(running(turns.length + 1, project, prompt));

  await page.route(/\/api\/events$/, (route) => {
    const said = unsent.map((turn) => {
      const event = { type: "chat_turn", turn, balance_micros: 0 };
      return `data: ${JSON.stringify(event)}\n\n`;
    });
    unsent = [];
    return route.fulfill({
      headers: { "content-type": "text/event-stream" },
      body: `retry: 250\n\n${said.join("")}`,
    });
  });
  await page.route(/\/api\/projects\/(\d+)\/chat$/, async (route, request) => {
    const project = projectOf(request.url());
    if (request.method() === "POST") {
      const { prompt } = request.postDataJSON() as { prompt: string };
      return route.fulfill({ status: 202, json: begin(project, prompt) });
    }
    // The real conversation's model and menu, with these turns in it.
    const real = await (await route.fetch()).json();
    const ours = turns.filter((turn) => turn.project === project);
    return route.fulfill({
      json: { ...real, session: ours.length > 0 ? SESSION : real.session, turns: ours },
    });
  });
  await page.route(/\/api\/projects\/(\d+)\/start$/, async (route, request) => {
    // The server keeps the new choice; only the turn it would start is ours.
    await route.fetch();
    const { message } = request.postDataJSON() as { message: string };
    return route.fulfill({ json: { turn: begin(projectOf(request.url()), message), note: null } });
  });
  await page.route(/\/api\/chat\/turns\/(\d+)\/quote$/, (route, request) => {
    const quoted = turns.find((turn) => request.url().includes(`/turns/${turn.id}/`));
    if (!quoted) return route.fulfill({ status: 404, json: { error: "no such turn" } });
    keep({ ...quoted, quote_answer: "confirmed" });
    const answered: Answered = {
      spent: null,
      refused: false,
      turn: begin(quoted.project, quoted.prompt),
      note: null,
    };
    return route.fulfill({ json: answered });
  });

  return {
    finish(turn, answer, quote) {
      keep({
        ...turn,
        state: "answered",
        answer,
        quote: quote ?? null,
        finished_at: now(),
      });
    },
    latest() {
      const last = turns.at(-1);
      if (!last) throw new Error("no turn has started");
      return last;
    },
  };
}

function projectOf(url: string): number {
  return Number(/projects\/(\d+)\//.exec(url)?.[1]);
}

const now = () => Math.floor(Date.now() / 1000);

/** A turn just started, as `POST /api/projects/{id}/chat` answers one. */
function running(id: number, project: number, prompt: string): TurnView {
  return {
    id,
    session: SESSION,
    project,
    prompt,
    state: "running",
    answer: null,
    stop_reason: null,
    model: "claude-sonnet-5-5",
    calls: 0,
    input_tokens: 0,
    output_tokens: 0,
    cache_write_tokens: 0,
    cache_read_tokens: 0,
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    questions: [],
    started_at: now(),
    finished_at: null,
  };
}
