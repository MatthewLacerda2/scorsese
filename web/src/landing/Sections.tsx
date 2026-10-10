// The landing page below the film: how it works, what it makes, what it
// handles, what it costs, and a last way in. Each section is moved by the
// scroll through the motion kit (`motion.ts`): what carries `data-rise` rises
// in as the section comes into view, staggered by `kit.stagger`.

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
import { type ReactNode, useRef } from "react";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { kit } from "@/landing/kit";
import { useScrollMotion } from "@/landing/motion";

/** A section that moves with the scroll, under an eyebrow and a title. */
function Section(props: { id?: string; eyebrow: string; title: string; children: ReactNode }) {
  const { language } = useLanguage();
  const root = useRef<HTMLElement>(null);
  useScrollMotion(root, language);
  return (
    <section
      ref={root}
      id={props.id}
      className="mx-auto max-w-6xl scroll-mt-8 px-4 py-20 sm:px-6 sm:py-28"
    >
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
      <div className="flex items-center justify-between">
        <Icon className="gold size-7" strokeWidth={1.5} />
        {props.step && <span className="mono dim text-sm">{props.step}</span>}
      </div>
      <h3 className="display mt-6 text-2xl font-medium">{props.title}</h3>
      <p className="dim mt-3 leading-relaxed">{props.body}</p>
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

export function WhatYouCanMake() {
  const t = useT().landing.make;
  const wide = [
    { picture: "/landing/promo.png", ...t.promo },
    { picture: "/landing/event.png", ...t.event },
  ];
  return (
    <Section eyebrow={t.eyebrow} title={t.title}>
      <div className="mt-12 grid gap-6 md:grid-cols-[2fr_1fr]">
        <div className="grid gap-6">
          {wide.map((item, index) => (
            <figure key={item.picture} className="card overflow-hidden" data-rise={at(index)}>
              <img
                src={item.picture}
                alt=""
                loading="lazy"
                width={960}
                height={540}
                className="aspect-video w-full object-cover"
              />
              <figcaption className="p-6">
                <h3 className="display text-2xl font-medium">{item.title}</h3>
                <p className="dim mt-2 leading-relaxed">{item.body}</p>
              </figcaption>
            </figure>
          ))}
        </div>
        <figure className="card overflow-hidden" data-rise={at(2)}>
          <img
            src="/landing/ad.png"
            alt=""
            loading="lazy"
            width={540}
            height={960}
            className="aspect-[9/16] w-full object-cover"
          />
          <figcaption className="p-6">
            <h3 className="display text-2xl font-medium">{t.ad.title}</h3>
            <p className="dim mt-2 leading-relaxed">{t.ad.body}</p>
          </figcaption>
        </figure>
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

export function Price() {
  const t = useT().landing.price;
  return (
    <Section eyebrow={t.eyebrow} title={t.title}>
      <div className="mt-10 grid items-end gap-10 md:grid-cols-2">
        <p className="dim max-w-xl text-lg leading-relaxed" data-rise="0.2">
          {t.body}
        </p>
        <div className="card p-6 sm:p-8" data-rise="0.3">
          <p className="dim text-sm leading-relaxed">{t.proof}</p>
          <dl className="mt-6 grid grid-cols-3 gap-4">
            <Figure count={21.6} decimals={1} label={t.seconds} />
            <Figure count={4} label={t.scenes} />
            <div>
              <dt className="sr-only">{t.cost}</dt>
              <dd className="mono gold text-3xl font-semibold tabular-nums sm:text-4xl">$0.00</dd>
              <dd className="dim mt-1 text-xs">{t.cost}</dd>
            </div>
          </dl>
        </div>
      </div>
    </Section>
  );
}

/** A figure that counts up as the section comes into view. */
function Figure(props: { count: number; decimals?: number; label: string }) {
  return (
    <div>
      <dt className="sr-only">{props.label}</dt>
      <dd
        className="mono text-3xl font-semibold tabular-nums sm:text-4xl"
        data-count={props.count}
        data-decimals={props.decimals ?? 0}
        data-at="0.3"
      >
        {props.count.toFixed(props.decimals ?? 0)}
      </dd>
      <dd className="dim mt-1 text-xs">{props.label}</dd>
    </div>
  );
}

export function Closing({ onSignIn }: { onSignIn: () => void }) {
  const t = useT().landing;
  const { language } = useLanguage();
  const root = useRef<HTMLElement>(null);
  useScrollMotion(root, language);
  return (
    <section ref={root} className="mx-auto max-w-6xl px-4 py-24 text-center sm:px-6 sm:py-32">
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
