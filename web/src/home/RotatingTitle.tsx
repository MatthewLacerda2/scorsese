// The home's title, which changes every few seconds: all of the titles sit in
// one cell of a grid (so the box never resizes) and the current one is the only
// one visible, the others faded out, so a change is a soft cross-fade. A visitor
// who asked for less motion gets the first title, standing.

import { useEffect, useState } from "react";
import { useT } from "@/i18n/I18nProvider";
import { prefersStill } from "@/lib/reducedMotion";

/** How long each title stays, in milliseconds. */
const EVERY_MS = 4500;

export function RotatingTitle() {
  const titles = useT().home.titles;
  const [current, setCurrent] = useState(0);
  useEffect(() => {
    if (prefersStill() || titles.length < 2) return;
    const id = setInterval(() => setCurrent((index) => (index + 1) % titles.length), EVERY_MS);
    return () => clearInterval(id);
  }, [titles.length]);

  return (
    <h1 className="grid font-heading text-3xl font-semibold text-balance sm:text-4xl">
      {titles.map((title, index) => (
        <span
          key={title}
          aria-hidden={index !== current}
          className={`col-start-1 row-start-1 transition-opacity duration-1000 motion-reduce:transition-none ${index === current ? "opacity-100" : "opacity-0"}`}
        >
          {title}
        </span>
      ))}
    </h1>
  );
}
