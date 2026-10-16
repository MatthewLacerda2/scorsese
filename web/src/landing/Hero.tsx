// The top of the landing page: the pitch, scorsese's name and mark beside it,
// and under them a strip of films made with scorsese (`Carousel`). The header
// above it (`Header.tsx`) carries only the language and the way in. The mark is the app's 512px icon
// (app/assets/icon.png) as a white silhouette with no background, made once:
//
//   ffmpeg -i app/assets/icon.png -filter_complex "[0]format=gray[a];
//     color=white:s=512x512,format=rgba[w];[w][a]alphamerge,format=rgba"
//     -frames:v 1 web/src/landing/media/mark.png
//
// Its brightness becomes its alpha.

import { useRef } from "react";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { Carousel } from "@/landing/Carousel";
import mark from "@/landing/media/mark.png";
import { useEntrance } from "@/landing/motion";

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
        {/* The mark and the name come up from behind an edge, not fading in (`data-emerge`). */}
        <a href="/" className="flex items-center gap-4 md:flex-col md:gap-5">
          <span className="block overflow-hidden">
            <img
              src={mark}
              alt=""
              width={140}
              height={140}
              className="block size-[4.375rem] md:size-35"
              data-emerge="0.45"
            />
          </span>
          <span className="block overflow-hidden pb-1">
            <span className="display block text-4xl font-semibold md:text-6xl" data-emerge="0.6">
              scorsese
            </span>
          </span>
        </a>
      </div>
      <div className="mt-12 sm:mt-16" data-rise="1">
        <Carousel label={t.landing.showcase.label} />
      </div>
    </section>
  );
}
