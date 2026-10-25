// A project's name from its prompt, in each of the site's languages.

import { expect, test } from "bun:test";
import { projectName } from "@/home/projectName";

const FALLBACK = "Novo vídeo 24/10";

test("English: the three longest words that are not stopwords, in prompt order", () => {
  expect(projectName("A 30-second Instagram launch video for my new running shoes", FALLBACK)).toBe(
    "Second Instagram running",
  );
});

test("Portuguese: stopwords and accents are handled, and the first letter is a capital", () => {
  expect(projectName("montagem de aniversário de 60 anos da minha mãe", FALLBACK)).toBe(
    "Montagem aniversário anos",
  );
  expect(projectName("um vídeo explicativo narrado sobre o nosso app", FALLBACK)).toBe(
    "Explicativo narrado app",
  );
});

test("Spanish: the same rule", () => {
  expect(projectName("Un recorrido por un departamento de dos habitaciones", FALLBACK)).toBe(
    "Recorrido departamento habitaciones",
  );
});

test("equally long words go by their place in the prompt", () => {
  expect(projectName("cats dogs bird fish", FALLBACK)).toBe("Cats dogs bird");
});

test("a single word is the name", () => {
  expect(projectName("Lighthouse", FALLBACK)).toBe("Lighthouse");
});

test("a prompt with no word worth a name gets the fallback", () => {
  expect(projectName("a la de the", FALLBACK)).toBe(FALLBACK);
  expect(projectName("   ", FALLBACK)).toBe(FALLBACK);
  expect(projectName("!!! ???", FALLBACK)).toBe(FALLBACK);
});

test("a name is never longer than 60 characters", () => {
  const long = "x".repeat(40);
  expect(projectName(`${long} ${long} ${long}`, FALLBACK).length).toBeLessThanOrEqual(60);
});
