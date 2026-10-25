// Every component of the assistant's panel mounted for real, effects and all,
// in every language (#1429). A static render never runs `useEffect`, and the
// end-to-end flows reach only what a faked turn draws; a translated message of
// the wrong shape — the 2026-10-16 black page (#1426) — crashes where it is
// used, so each component is mounted where it would be used.
//
// `MOUNTS` names a mount for each component file in this folder, and the
// first test holds that list to the folder: a new component is mounted here
// or this fails.

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render } from "@testing-library/react";
import { Glob } from "bun";
import type { ReactNode } from "react";
import type { Conversation, ModelChoice, QuestionView, TurnView } from "@/api/chat";
import { setFetch } from "@/api/client";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import { LANGUAGES } from "@/i18n/language";
import { noAttachments } from "@/test/attachments";
import { picker } from "@/test/picker";
import { Busy } from "./Busy";
import { ChatPanel } from "./ChatPanel";
import { Composer } from "./Composer";
import { EffortPicker } from "./EffortPicker";
import { Generating } from "./Generating";
import { ModelPicker } from "./ModelPicker";
import { NewConversation } from "./NewConversation";
import { PickerCard } from "./PickerCard";
import { PickerModal } from "./PickerModal";
import { QuestionCard } from "./QuestionCard";
import { QuoteBox } from "./QuoteBox";
import { Reply } from "./Reply";
import { Turn } from "./Turn";

const PROJECT = 7;

const turn = (id: number, change: Partial<TurnView> = {}): TurnView => ({
  id,
  session: 1,
  project: PROJECT,
  prompt: "Cut the intro",
  state: "running",
  answer: null,
  stop_reason: null,
  model: "m",
  calls: 1,
  input_tokens: 0,
  output_tokens: 0,
  cache_write_tokens: 0,
  cache_read_tokens: 0,
  charged_micros: 1_200,
  quote: null,
  quote_answer: null,
  quote_replies: [],
  woken: false,
  questions: [],
  started_at: 1,
  finished_at: null,
  ...change,
});

const quote = { tool: "generate_video", lines: ["One shot"], micros: 3_200_000, expires_at: 2e9 };
const question: QuestionView = { question: "Which?", options: ["A", "B"], answer: null };

/** A turn of each kind a conversation holds: running, waiting on a quote, asking. */
const turns = [
  turn(1, { state: "answered", answer: "Done.", quote, finished_at: 2 }),
  turn(2, { state: "asking", questions: [question] }),
  turn(3),
];
const models: ModelChoice[] = [
  { id: "m", label: "M", vendor: "anthropic", unavailable: null, cache_seconds: 300, cost: null },
];
const conversation: Conversation = {
  project: PROJECT,
  session: 1,
  turns,
  model: "m",
  models,
  effort: "medium",
};
const entry = (of: TurnView) => ({ turn: of, streamed: "", lines: [], balanceAfter: null });
const ignore = () => {};

/** Each component of this folder, by its file's name, mounted as it is used. */
const MOUNTS: Record<string, () => ReactNode> = {
  Busy: () => <Busy />,
  ChatPanel: () => (
    <ChatPanel projectId={PROJECT} full={false} onFull={ignore} attachments={noAttachments} />
  ),
  Composer: () => (
    <Composer
      projectId={PROJECT}
      column=""
      runningTurn={null}
      askingTurn={null}
      fresh={false}
      onFresh={ignore}
      showFresh
      onStarted={ignore}
      attachments={noAttachments}
    />
  ),
  EffortPicker: () => <EffortPicker projectId={PROJECT} effort="medium" />,
  Generating: () => <Generating />,
  ModelPicker: () => <ModelPicker projectId={PROJECT} model="m" models={models} turns={turns} />,
  NewConversation: () => <NewConversation onStart={ignore} />,
  PickerCard: () => <PickerCard turn={2} question={picker} waiting />,
  PickerModal: () => <PickerModal turn={2} question={picker} open onOpenChange={ignore} />,
  QuestionCard: () => <QuestionCard turn={2} question={question} waiting />,
  QuoteBox: () => <QuoteBox turn={1} project={PROJECT} quote={quote} />,
  Reply: () => <Reply text={"**Done.** A [link](https://example.invalid)."} />,
  Turn: () => (
    <ol>
      {turns.map((each) => (
        <Turn key={each.id} entry={entry(each)} />
      ))}
    </ol>
  ),
};

let previous: Parameters<typeof setFetch>[0];
const realEventSource = globalThis.EventSource;

beforeAll(() => {
  // The project the quote box colours its items from; every other call reads the conversation.
  previous = setFetch(async (input) =>
    input.endsWith(`/projects/${PROJECT}`)
      ? new Response(JSON.stringify({ id: PROJECT, revision: 1, document: { assets: [] } }))
      : new Response(JSON.stringify(conversation)),
  );
  // The live stream is never opened for real: a source that stays silent.
  globalThis.EventSource = class {
    close() {}
  } as unknown as typeof EventSource;
});

afterAll(() => {
  setFetch(previous);
  globalThis.EventSource = realEventSource;
});

afterEach(cleanup);

test("every component in this folder has a mount", async () => {
  const files: string[] = [];
  for await (const path of new Glob("*.tsx").scan(import.meta.dir)) {
    if (!path.endsWith(".test.tsx")) files.push(path.replace(/\.tsx$/, ""));
  }
  expect(Object.keys(MOUNTS).sort()).toEqual(files.sort());
});

describe.each(LANGUAGES.map(({ language }) => language))("%s", (language) => {
  test.each(Object.keys(MOUNTS))("%s mounts, runs its effects and stays drawn", async (name) => {
    const mount = MOUNTS[name] as () => ReactNode;
    const { baseElement } = render(
      <QueryClientProvider client={createQueryClient()}>
        <I18nProvider initial={language}>{mount()}</I18nProvider>
      </QueryClientProvider>,
    );
    // Effects, the conversation's fetch, and whatever they set off.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));
    });
    expect(baseElement.textContent?.trim()).not.toBe("");
  });
});
