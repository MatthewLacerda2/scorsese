// Asking the assistant something, with its reply faked in the browser: the
// one request a real turn would spend money on never leaves the page. What is
// checked is the page's half — the message goes out, and the turn the server
// would answer with is drawn.

import { expect, test } from "@playwright/test";
import { newProject } from "./session";

const REPLY = "Here is a cut with the lighthouse first.";

test("a message to the assistant shows its reply", async ({ page }) => {
  await newProject(page, "Assistant");
  let asked: unknown = null;
  await page.route(/\/api\/projects\/(\d+)\/chat$/, async (route, request) => {
    if (request.method() !== "POST") return route.fallback();
    asked = request.postDataJSON();
    const project = Number(/projects\/(\d+)\/chat/.exec(request.url())?.[1]);
    await route.fulfill({ json: turn(project, (asked as { prompt: string }).prompt) });
  });

  await page.getByRole("textbox").last().fill("Put the lighthouse first");
  await page.keyboard.press("Enter");

  await expect(page.getByText(REPLY)).toBeVisible();
  expect(asked).toMatchObject({ prompt: "Put the lighthouse first" });
});

/** A finished turn, as `POST /api/projects/{id}/chat` returns one. */
function turn(project: number, prompt: string) {
  return {
    id: 1,
    session: 1,
    project,
    prompt,
    state: "answered",
    answer: REPLY,
    stop_reason: null,
    model: "claude-sonnet-5-5",
    calls: 1,
    input_tokens: 0,
    output_tokens: 0,
    cache_write_tokens: 0,
    cache_read_tokens: 0,
    charged_micros: 0,
    quote: null,
    quote_answer: null,
    questions: [],
    started_at: Math.floor(Date.now() / 1000),
    finished_at: Math.floor(Date.now() / 1000),
  };
}
