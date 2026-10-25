// The prompt box's placeholder, typed like a person would: it starts empty,
// types the first example a character at a time, holds it, deletes it a
// character at a time (faster than it typed), then types the next, round and
// round. While `active` is false (the box has text) no timer runs, and it
// starts over from the beginning of the current example when it is true again.
// A visitor who asked for less motion gets the first example, standing.

import { useEffect, useRef, useState } from "react";
import { prefersStill } from "@/lib/reducedMotion";

/** Milliseconds: a pause before the first character, per typed one, per deleted one, a full example held, and an empty box before the next. */
export const PACE = { start: 500, type: 65, erase: 30, hold: 2400, gap: 450 };

/** `base` stretched or squeezed by up to 40% (`random()` is 0 to 1, so 0.5 is exact). */
function jittered(base: number, random: () => number): number {
  return base * (0.8 + 0.4 * random());
}

export function useTypewriter(
  examples: readonly string[],
  active: boolean,
  random: () => number = Math.random,
): string {
  const still = prefersStill();
  const [text, setText] = useState("");
  const index = useRef(0);

  useEffect(() => {
    if (still || !active || examples.length === 0) return;
    let timer: ReturnType<typeof setTimeout>;
    let length = 0;
    const example = () => examples[index.current % examples.length] ?? "";
    const show = (next: number) => {
      length = next;
      setText(example().slice(0, length));
    };
    const erase = () => {
      show(length - 1);
      if (length > 0) timer = setTimeout(erase, jittered(PACE.erase, random));
      else {
        index.current = (index.current + 1) % examples.length;
        timer = setTimeout(type, PACE.gap);
      }
    };
    const type = () => {
      show(length + 1);
      if (length < example().length) timer = setTimeout(type, jittered(PACE.type, random));
      else timer = setTimeout(erase, PACE.hold);
    };
    setText("");
    timer = setTimeout(type, PACE.start);
    return () => clearTimeout(timer);
  }, [examples, active, still, random]);

  return still ? (examples[0] ?? "") : text;
}
