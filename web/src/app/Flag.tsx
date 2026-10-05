// The flag beside each language in the picker (#770): recognised faster than a
// word, by someone who cannot read the page's language yet.
//
// Drawn as inline SVG, never emoji: Windows draws no flag emoji, so 🇧🇷 would
// read "BR" there. Simplified to what reads at twelve pixels tall — Brazil's
// stars and motto, the United States' fifty stars, Spain's arms are dropped.
// Nothing is fetched: the web app's pages are served by us alone.

import type { ReactNode } from "react";
import type { Language } from "@/i18n/language";

const SHAPES: Record<Language, ReactNode> = {
  "pt-BR": (
    <>
      <rect width="18" height="12" fill="#009b3a" />
      <path d="M9 1.5 16.5 6 9 10.5 1.5 6z" fill="#fedf00" />
      <circle cx="9" cy="6" r="2.6" fill="#002776" />
    </>
  ),
  es: (
    <>
      <rect width="18" height="12" fill="#aa151b" />
      <rect y="3" width="18" height="6" fill="#f1bf00" />
    </>
  ),
  en: (
    <>
      <rect width="18" height="12" fill="#fff" />
      {[0, 2, 4, 6, 8, 10, 12].map((stripe) => (
        <rect key={stripe} y={(stripe * 12) / 13} width="18" height={12 / 13} fill="#b22234" />
      ))}
      <rect width="7.6" height={(7 * 12) / 13} fill="#3c3b6e" />
    </>
  ),
};

/** The flag of a language's country — decoration beside its name. */
export function Flag({ language }: { language: Language }) {
  // The wrapper sets the size: `size-full` keeps the select's `size-4` rule for
  // icons from squaring the flag.
  return (
    <span className="inline-flex h-3 w-[18px] shrink-0 overflow-hidden rounded-[2px] ring-1 ring-foreground/15">
      <svg
        viewBox="0 0 18 12"
        className="size-full"
        preserveAspectRatio="none"
        aria-hidden="true"
        data-flag={language}
      >
        {SHAPES[language]}
      </svg>
    </span>
  );
}
