// What scorsese handles, as a row of cards: a slide of the landing page
// (`HowItWorks.tsx` says how they move).

import { Gauge, Mic, Music, Scissors, Type } from "lucide-react";
import { useT } from "@/i18n/I18nProvider";
import { Feature } from "./Feature";
import { Section } from "./Section";

export function WhatItHandles() {
  const t = useT().landing.handles;
  const items = [
    { icon: Music, ...t.music },
    { icon: Type, ...t.titles },
    { icon: Scissors, ...t.cuts },
    { icon: Gauge, ...t.pacing },
    { icon: Mic, ...t.narration },
  ];
  return (
    <Section eyebrow={t.eyebrow} title={t.title}>
      <div className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        {items.map((item, index) => (
          <Feature key={item.title} {...item} index={index} />
        ))}
      </div>
    </Section>
  );
}
