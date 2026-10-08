// A Lottie in the picker (#908): it is shown moving, by its GIF, in the grid
// and enlarged alike — never as a video player, since what it imports is a
// file under pages/ — and its tile names LottieFiles, beside Pixabay's results.
// Mounted in happy-dom (src/test/dom.ts).

import { expect, test } from "bun:test";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { CandidateView, QuestionView } from "@/api/chat";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { PickerModal } from "./PickerModal";

const t = CATALOGUES.en.chat.picker;

const wave: CandidateView = {
  key: "lottiefiles-lottie-8",
  source: "lottiefiles",
  kind: "lottie",
  id: "8",
  preview_url: "https://cdn.example.invalid/8.gif",
  look_url: "https://cdn.example.invalid/8.gif",
  width: 512,
  height: 512,
  seconds: 2,
  author: "someone",
  page_url: "https://lottiefiles.com/free-animation/wave-8",
  tags: ["wave"],
};

const sunrise: CandidateView = {
  ...wave,
  key: "pixabay-image-1",
  source: "pixabay",
  kind: "image",
  id: "1",
  preview_url: "https://cdn.example.invalid/1_640.jpg",
  look_url: "https://cdn.example.invalid/1_1280.jpg",
  seconds: null,
  tags: ["sunrise"],
};

const question: QuestionView = {
  question: "Which opening?",
  options: [],
  answer: null,
  candidates: [wave, sunrise],
};

function mount() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <I18nProvider initial="en">
        <PickerModal turn={1} question={question} open onOpenChange={() => {}} />
      </I18nProvider>
    </QueryClientProvider>,
  );
}

test("a Lottie's tile moves by its GIF and names LottieFiles", () => {
  mount();
  const [lottie, image] = screen.getAllByRole("button", { name: t.enlarge }) as [
    HTMLElement,
    HTMLElement,
  ];
  expect(within(lottie).getByRole("img").getAttribute("src")).toBe(wave.preview_url);
  expect(within(image).getByRole("img").getAttribute("src")).toBe(sunrise.preview_url);
  expect(screen.getAllByText("LottieFiles")).toHaveLength(1);
  expect(screen.getByText(t.from("LottieFiles, Pixabay"))).toBeTruthy();
});

test("enlarged, a Lottie is its GIF, not a video", async () => {
  mount();
  const [lottie] = screen.getAllByRole("button", { name: t.enlarge }) as [HTMLElement];
  await userEvent.click(lottie);
  expect(document.querySelector("video")).toBeNull();
  const shown = screen.getByRole("img", { name: "wave" });
  expect(shown.getAttribute("src")).toBe(wave.look_url);
});
