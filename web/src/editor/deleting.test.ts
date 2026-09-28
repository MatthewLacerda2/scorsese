// The Delete key fires on a selected clip, and never while somebody types.

import { describe, expect, test } from "bun:test";
import { deletes, isTyping } from "./deleting";

describe("the delete key", () => {
  test("is Delete, or Backspace for a Mac keyboard", () => {
    expect(deletes("Delete")).toBe(true);
    expect(deletes("Backspace")).toBe(true);
    expect(deletes("d")).toBe(false);
  });

  test("is typing in a field, and a command anywhere else", () => {
    expect(isTyping({ tagName: "INPUT" })).toBe(true);
    expect(isTyping({ tagName: "TEXTAREA" })).toBe(true);
    expect(isTyping({ tagName: "DIV", isContentEditable: true })).toBe(true);
    // A clip on the timeline is a button, and focusing it is how it was picked.
    expect(isTyping({ tagName: "BUTTON" })).toBe(false);
    expect(isTyping({ tagName: "BODY" })).toBe(false);
    expect(isTyping(null)).toBe(false);
  });
});
