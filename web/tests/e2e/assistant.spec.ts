// The flows that run an assistant turn, in every language the web app speaks
// (#1429): a message and a confirmed quote. Each checks the page
// still shows the editor while the turn runs and after it ends, because the
// pieces a running turn draws are translated and stateful — the 2026-10-16
// black page (#1426) broke them in Portuguese and Spanish only, where no
// English flow could see it. The turns are faked in the browser
// (`fakeAssistant.ts`); the rest of the app is the real server.
//
// Only these flows run three times: the others draw nothing a language
// changes the shape of, and `make web-e2e` should not triple for them.

import { expect, type Page, test } from "@playwright/test";
import { CATALOGUES, type Messages } from "@/i18n/catalogue";
import { LANGUAGES, type Language } from "@/i18n/language";
import { KEYS } from "@/lib/storage";
import { type FakeAssistant, fakeAssistant } from "./fakeAssistant";
import { newProject } from "./session";

const REPLY = "Here is a cut with the lighthouse first.";

for (const { language } of LANGUAGES) {
  const t = CATALOGUES[language];

  test.describe(`in ${language}`, () => {
    test("a message to the assistant runs, then shows its reply", async ({ page }) => {
      const assistant = await editing(page, language, "Assistant");
      await page.getByPlaceholder(t.chat.composer.placeholder).fill("Put the lighthouse first");
      await page.keyboard.press("Enter");
      await runsThenAnswers(page, t, assistant);
    });

    test("a confirmed quote carries the turn on, running, then answered", async ({ page }) => {
      const assistant = await editing(page, language, "Quote");
      await page.getByPlaceholder(t.chat.composer.placeholder).fill("Make the opening shot");
      await page.keyboard.press("Enter");
      await expect(busy(page, t)).toBeVisible();
      assistant.finish(assistant.latest(), "That shot costs money.", QUOTE);
      await page.getByRole("button", { name: t.chat.quote.confirm, exact: true }).click();
      await runsThenAnswers(page, t, assistant);
    });
  });
}

/**
 * A new project's editor with the assistant faked, in `language`: made in
 * English, then switched the way a visitor switches — the stored choice
 * Settings writes — and reloaded.
 */
async function editing(page: Page, language: Language, name: string): Promise<FakeAssistant> {
  const assistant = await fakeAssistant(page);
  await newProject(page, name);
  await page.evaluate(({ key, value }) => localStorage.setItem(key, value), {
    key: KEYS.language,
    value: language,
  });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await editorShown(page, CATALOGUES[language]);
  return assistant;
}

/** The latest turn is drawn running, then answered — the editor whole throughout. */
async function runsThenAnswers(page: Page, t: Messages, assistant: FakeAssistant) {
  await expect(busy(page, t)).toBeVisible();
  await editorShown(page, t);
  assistant.finish(assistant.latest(), REPLY);
  await expect(page.getByText(REPLY)).toBeVisible();
  await expect(busy(page, t)).toBeHidden();
  await editorShown(page, t);
}

/** The editor, not an empty root or a page-wide error. */
async function editorShown(page: Page, t: Messages) {
  // The editor's title is the page's one top-level heading; panels add lower ones.
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(page.getByPlaceholder(t.chat.composer.placeholder)).toBeVisible();
}

/** A running turn's status line: whichever of its words it is on. */
function busy(page: Page, t: Messages) {
  const words = t.chat.turn.busy.map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  return page.getByText(new RegExp(`^\\s*(${words.join("|")})\\s*$`));
}

/** A paid tool's quote, waiting for a yes. */
const QUOTE = {
  tool: "generate_video",
  lines: ["One Veo shot, 8 seconds."],
  micros: 3_200_000,
  expires_at: Math.floor(Date.now() / 1000) + 3600,
};
