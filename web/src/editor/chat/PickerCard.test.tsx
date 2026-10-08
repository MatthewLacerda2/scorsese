// An answered picker shrinks to its question and what was picked: the
// pictures, or "none", and any words written beside them (#901).

import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import type { CandidateView, QuestionView } from "@/api/chat";
import { CATALOGUES } from "@/i18n/catalogue";
import { I18nProvider } from "@/i18n/I18nProvider";
import { PickerCard } from "./PickerCard";

function candidate(id: number): CandidateView {
  return {
    ...({} as CandidateView),
    key: `pixabay-image-${id}`,
    source: "pixabay",
    kind: "image",
    id: String(id),
    preview_url: `https://cdn.example.invalid/${id}.jpg`,
    tags: ["sunrise"],
  };
}

const picker: QuestionView = {
  question: "Which sunrise?",
  options: [],
  answer: null,
  candidates: [candidate(1), candidate(2), candidate(3)],
};

function html(question: QuestionView): string {
  return renderToString(
    <I18nProvider initial="en">
      <PickerCard turn={1} question={question} waiting={false} />
    </I18nProvider>,
  );
}

test("an answered picker shows the pictures picked", () => {
  const page = html({ ...picker, picked: ["pixabay-image-3"] });
  expect(page).toContain("Which sunrise?");
  expect(page).toContain("https://cdn.example.invalid/3.jpg");
  expect(page).not.toContain("https://cdn.example.invalid/1.jpg");
});

test("none picked says so, with the words written instead", () => {
  const page = html({ ...picker, picked: [], answer: "something darker" });
  expect(page).toContain(CATALOGUES.en.chat.picker.noneTaken);
  expect(page).toContain("something darker");
});
