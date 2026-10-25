// A picker with one stock candidate, for a test that mounts what shows one.

import type { QuestionView } from "@/api/chat";

export const picker: QuestionView = {
  question: "Which shot?",
  options: [],
  answer: null,
  candidates: [
    {
      key: "pixabay-image-1",
      source: "pixabay",
      kind: "image",
      id: "1",
      preview_url: "https://cdn.example.invalid/1.jpg",
      look_url: "https://cdn.example.invalid/1.jpg",
      width: 640,
      height: 360,
      seconds: null,
      author: "someone",
      page_url: "https://pixabay.com/photos/1",
      tags: ["sea"],
    },
  ],
};
