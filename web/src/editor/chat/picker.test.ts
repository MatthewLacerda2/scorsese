import { describe, expect, test } from "bun:test";
import type { CandidateView, QuestionView, TurnView } from "@/api/chat";
import { isPicker, length, pickedOf, sourceName, toggle } from "./picker";
import { waitingQuestion } from "./transcript";

function candidate(id: number, kind = "image"): CandidateView {
  return {
    key: `pixabay-${kind}-${id}`,
    source: "pixabay",
    kind,
    id: String(id),
    preview_url: `https://cdn.example.invalid/${id}.jpg`,
    look_url: `https://cdn.example.invalid/${id}.mp4`,
    width: 1920,
    height: 1080,
    seconds: kind === "video" ? 12 : null,
    author: "someone",
    page_url: `https://pixabay.com/id-${id}/`,
    tags: ["sunrise"],
  };
}

const offered = [candidate(1), candidate(2, "video"), candidate(3)];
const picker: QuestionView = {
  question: "Which sunrise?",
  options: [],
  answer: null,
  candidates: offered,
};

describe("isPicker", () => {
  test("is a question with candidates", () => {
    expect(isPicker(picker)).toBe(true);
    expect(isPicker({ question: "Loop or fade?", options: ["loop", "fade"], answer: null })).toBe(
      false,
    );
  });
});

describe("toggle", () => {
  test("keeps the selection in the order the candidates are offered", () => {
    let selected = toggle([], "pixabay-image-3", offered);
    selected = toggle(selected, "pixabay-image-1", offered);
    expect(selected).toEqual(["pixabay-image-1", "pixabay-image-3"]);
    expect(toggle(selected, "pixabay-image-1", offered)).toEqual(["pixabay-image-3"]);
  });
});

describe("pickedOf", () => {
  test("is what was picked, and nothing for none of them", () => {
    const answered = { ...picker, picked: ["pixabay-image-3"] };
    expect(pickedOf(answered).map((one) => one.id)).toEqual(["3"]);
    expect(pickedOf({ ...picker, picked: [] })).toEqual([]);
  });
});

describe("waitingQuestion", () => {
  test("is a picker until it is picked from, even with no words beside the pick", () => {
    const turn = { state: "asking", questions: [picker] } as unknown as TurnView;
    expect(waitingQuestion(turn)).toEqual(picker);
    const picked = { ...turn, questions: [{ ...picker, picked: [] }] } as TurnView;
    expect(waitingQuestion(picked)).toBeNull();
  });
});

describe("length and source", () => {
  test("read as a badge and a credit", () => {
    expect(length(8)).toBe("0:08");
    expect(length(90)).toBe("1:30");
    expect(length(null)).toBeNull();
    expect(sourceName(offered)).toBe("Pixabay");
    const lottie = { ...offered[0], source: "lottiefiles" } as CandidateView;
    expect(sourceName([...offered, lottie])).toBe("Pixabay, LottieFiles");
  });
});
