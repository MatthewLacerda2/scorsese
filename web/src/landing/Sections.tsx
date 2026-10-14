// The landing page below the top: how it works, what it handles, and a last
// way in. Each is a slide (`Slides.tsx`): what carries
// `data-rise` rises in as the slide fades in, staggered by `kit.stagger`.

import {
  Clapperboard,
  FolderUp,
  Gauge,
  type LucideIcon,
  MessageSquareText,
  Mic,
  Music,
  Scissors,
  Type,
} from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "@/i18n/I18nProvider";
import { kit } from "@/landing/kit";

/** A section that moves with the scroll, under an eyebrow and a title. */
function Section(props: { id?: string; eyebrow: string; title: string; children: ReactNode }) {
  return (
    <section id={props.id} className="mx-auto max-w-6xl scroll-mt-8 px-4 py-20 sm:px-6 sm:py-28">
      <p className="eyebrow" data-rise="0.05">
        {props.eyebrow}
      </p>
      <h2
        className="display mt-4 max-w-3xl text-4xl leading-tight font-medium sm:text-5xl"
        data-rise="0.1"
      >
        {props.title}
      </h2>
      {props.children}
    </section>
  );
}

/** When the `index`th item of a row rises in: one after another, from 0.2. */
const at = (index: number) => String(kit.stagger(index, 0.06, 0.2));

function Feature(props: {
  icon: LucideIcon;
  title: string;
  body: string;
  index: number;
  step?: string;
}) {
  const Icon = props.icon;
  return (
    <div className="card p-6 sm:p-8" data-rise={at(props.index)}>
      <div className="flex items-center gap-3">
        <Icon className="gold size-7 shrink-0" strokeWidth={1.5} />
        <h3 className="display text-2xl leading-tight font-medium">{props.title}</h3>
        {props.step && <span className="mono dim ml-auto text-sm">{props.step}</span>}
      </div>
      <p className="dim mt-4 leading-relaxed">{props.body}</p>
    </div>
  );
}

export function HowItWorks() {
  const t = useT().landing.how;
  const steps = [
    { icon: FolderUp, ...t.files },
    { icon: MessageSquareText, ...t.describe },
    { icon: Clapperboard, ...t.adjust },
  ];
  return (
    <Section id="how" eyebrow={t.eyebrow} title={t.title}>
      <div className="mt-12 grid gap-4 md:grid-cols-3">
        {steps.map((step, index) => (
          <Feature key={step.title} {...step} index={index} step={`0${index + 1}`} />
        ))}
      </div>
    </Section>
  );
}

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

export function Closing({ onSignIn }: { onSignIn: () => void }) {
  const t = useT().landing;
  return (
    <section className="mx-auto max-w-6xl px-4 py-24 text-center sm:px-6 sm:py-32">
      <h2 className="display text-5xl font-medium italic sm:text-7xl" data-rise="0.1">
        {t.closing.title}
      </h2>
      <p className="dim mx-auto mt-6 max-w-xl text-lg" data-rise="0.2">
        {t.closing.body}
      </p>
      <div className="mt-10" data-rise="0.3">
        <button type="button" onClick={onSignIn} className="cta px-8 py-3.5 text-base">
          {t.signIn}
        </button>
      </div>
    </section>
  );
}
