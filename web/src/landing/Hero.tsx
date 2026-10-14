// The top of the landing page: the pitch, scorsese's name and mark beside it,
// and under them a strip of films made with scorsese (`Carousel`). The header
// above it carries only the language and the way in, since the name is
// already the largest thing on the page. The mark is the app's own 512px icon
// (app/assets/), so it stays sharp at any size the page draws it.

import { useRef } from "react";
import { LanguageControl } from "@/app/LanguageControl";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { Carousel } from "@/landing/Carousel";
import { useEntrance } from "@/landing/motion";
import icon from "../../../app/assets/icon.png";

export function Header({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  return (
    <header className="dark fixed inset-x-0 top-0 z-20 mx-auto flex max-w-6xl items-center justify-end gap-2 px-4 py-5 sm:gap-3 sm:px-6">
      <LanguageControl className="border-[#f4efe6]/15 bg-transparent" />
      <button type="button" onClick={onSignIn} className="ghost px-4 py-1.5 text-sm">
        {t.landing.signIn}
      </button>
    </header>
  );
}

export function Hero({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  const { language } = useLanguage();
  const root = useRef<HTMLElement>(null);
  useEntrance(root, language);

  return (
    <section ref={root} className="relative pt-10 pb-16 sm:pt-16">
      <div className="mx-auto grid max-w-6xl items-center gap-10 px-4 sm:px-6 md:grid-cols-[1fr_auto]">
        <div>
          <p className="eyebrow" data-rise="0">
            {t.landing.hero.eyebrow}
          </p>
          <h1 className="display mt-5 text-5xl text-balance leading-[1.02] font-semibold sm:text-7xl">
            <span className="block" data-rise="0.1">
              {t.landing.hero.title}
            </span>
            <span className="gold block italic" data-rise="0.35">
              {t.landing.hero.titleEnd}
            </span>
          </h1>
          <p className="dim mt-6 max-w-2xl text-lg leading-relaxed sm:text-xl" data-rise="0.6">
            {t.landing.hero.lead}
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-4" data-rise="0.8">
            <button type="button" onClick={onSignIn} className="cta px-7 py-3 text-base">
              {t.landing.signIn}
            </button>
            <a href="#how" className="dim px-2 py-3 text-base underline-offset-4 hover:underline">
              {t.landing.hero.scroll} ↓
            </a>
          </div>
        </div>
        {/* Raised by half the sign-in button's height (24px): a centred grid item moves by half its negative margin. */}
        <a
          href="/"
          className="flex items-center gap-4 md:-mt-12 md:flex-col md:gap-5"
          data-rise="0.5"
        >
          <img
            src={icon}
            alt=""
            width={112}
            height={112}
            className="size-14 rounded-2xl md:size-28"
          />
          <span className="display text-4xl font-semibold md:text-6xl">scorsese</span>
        </a>
      </div>
      <div className="mt-12 sm:mt-16" data-rise="1">
        <Carousel label={t.landing.showcase.label} />
      </div>
    </section>
  );
}
