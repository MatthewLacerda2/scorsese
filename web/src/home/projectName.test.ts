// A project's name from its prompt, in each of the site's languages (#1535).

import { expect, test } from "bun:test";
import { projectName } from "@/home/projectName";

const FALLBACK = "Novo vídeo 24/10";

test("Portuguese: the issue's own example", () => {
  expect(
    projectName(
      "quero um vídeo de lançamento da minha bicicleta elétrica para o instagram",
      FALLBACK,
    ),
  ).toBe("Lançamento Bicicleta Instagram");
  expect(projectName("montagem de aniversário de 60 anos da minha mãe", FALLBACK)).toBe(
    "Montagem Aniversário Anos",
  );
});

test("English: numbers, punctuation and the words every prompt has are left out", () => {
  expect(projectName("I want to make a 30-second video for my new running shoes!", FALLBACK)).toBe(
    "Second Running Shoes",
  );
});

test("Spanish: the same rule", () => {
  expect(
    projectName("Quiero hacer un recorrido por un departamento de dos habitaciones", FALLBACK),
  ).toBe("Recorrido Departamento Habitaciones");
});

test("a word is taken once, however it is written", () => {
  expect(projectName("Beach beach BEACH sunset waves", FALLBACK)).toBe("Beach Sunset Waves");
});

test("equally long words go by their place in the prompt", () => {
  expect(projectName("cats dogs bird fish", FALLBACK)).toBe("Cats Dogs Bird");
});

test("a single word is the name", () => {
  expect(projectName("Lighthouse", FALLBACK)).toBe("Lighthouse");
});

test("a prompt with no word worth a name gets the fallback", () => {
  expect(projectName("a la de the", FALLBACK)).toBe(FALLBACK);
  expect(projectName("quero fazer um vídeo", FALLBACK)).toBe(FALLBACK);
  expect(projectName("   ", FALLBACK)).toBe(FALLBACK);
  expect(projectName("!!! ??? 2024", FALLBACK)).toBe(FALLBACK);
});

test("a name is at most 40 characters, and loses whole words to get there", () => {
  expect(projectName("internationalisation incomprehensibilities", FALLBACK)).toBe(
    "Internationalisation",
  );
  expect(projectName("x".repeat(50), FALLBACK)).toBe(`X${"x".repeat(39)}`);
});
