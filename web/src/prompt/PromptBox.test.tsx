// The prompt box's keys: Enter sends, Shift+Enter is a new line, empty sends nothing.

import { expect, test } from "bun:test";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createQueryClient } from "@/app/queryClient";
import { I18nProvider } from "@/i18n/I18nProvider";
import { PromptBox } from "@/prompt/PromptBox";

function setup(answer?: string) {
  const sent: string[] = [];
  const client = createQueryClient();
  // What the "+" takes, so focusing the box asks nothing.
  client.setQueryData(["formats"], { video: [], image: [], audio: [], html: [], text: [] });
  render(
    <QueryClientProvider client={client}>
      <I18nProvider initial="en">
        <PromptBox
          onSubmit={(text) => {
            sent.push(text);
            return answer;
          }}
        />
      </I18nProvider>
    </QueryClientProvider>,
  );
  return { sent, box: screen.getByRole("textbox") };
}

test("Enter sends the text, trimmed", async () => {
  const { sent, box } = setup();
  await userEvent.type(box, "  a trailer  {Enter}");
  expect(sent).toEqual(["a trailer"]);
});

test("Shift+Enter is a new line and sends nothing", async () => {
  const { sent, box } = setup();
  await userEvent.type(box, "one{Shift>}{Enter}{/Shift}two");
  expect(sent).toEqual([]);
  expect((box as HTMLTextAreaElement).value).toBe("one\ntwo");
});

test("an empty box sends nothing, from the key or the button", async () => {
  const { sent, box } = setup();
  await userEvent.type(box, "   {Enter}");
  expect(sent).toEqual([]);
  expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
});

test("the sentence the handler answers with is shown in red under the box", async () => {
  const { box } = setup("nope");
  expect(screen.queryByRole("alert")).toBeNull();
  await userEvent.type(box, "hi{Enter}");
  const alert = screen.getByRole("alert");
  expect(alert.textContent).toBe("nope");
  expect(alert.className).toContain("text-destructive");
});
