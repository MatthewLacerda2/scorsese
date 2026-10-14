// The top of the landing page: the name, the pitch, and the film scorsese
// rendered of itself (web/landing/hero.scor, `make landing-hero`). The film is
// muted, plays on its own and loops; it never blocks the first paint, since a
// `<video>` downloads beside the page rather than before it. A visitor who
// asked for less motion gets its poster and the player's controls instead.
//
// The film and its poster are imported, so Vite fingerprints them under
// `/assets/` and nginx serves them `immutable` (#1238): a second visit takes
// them from cache, and a re-render by `make landing-hero` is a new name.

import { Volume2, VolumeX } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { LanguageControl } from "@/app/LanguageControl";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { kit } from "@/landing/kit";
import film from "@/landing/media/hero.mp4";
import poster from "@/landing/media/poster.png";
import { prefersStill, useEntrance, useOnScroll } from "@/landing/motion";

export function Header({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  return (
    <header className="dark fixed inset-x-0 top-0 z-20 mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-5 sm:px-6">
      <a href="/" className="flex items-center gap-3">
        <img src="/icon.png" alt="" width={36} height={36} className="size-9 rounded-lg" />
        <span className="display text-2xl font-semibold">scorsese</span>
      </a>
      <div className="flex items-center gap-2 sm:gap-3">
        <LanguageControl className="border-[#f4efe6]/15 bg-transparent" />
        <button type="button" onClick={onSignIn} className="ghost px-4 py-1.5 text-sm">
          {t.landing.signIn}
        </button>
      </div>
    </header>
  );
}

export function Hero({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  const { language } = useLanguage();
  const root = useRef<HTMLElement>(null);
  const screen = useRef<HTMLDivElement>(null);
  const video = useRef<HTMLVideoElement>(null);
  const [still] = useState(prefersStill);
  const [muted, setMuted] = useState(true);
  useEntrance(root, language);

  // As the page scrolls away from it, the film settles back and dims.
  const recede = useCallback(() => {
    if (!screen.current || still) return;
    const away = scrollY / Math.max(innerHeight, 1);
    screen.current.style.transform = `scale(${kit.remap(away, 0, 1, 1, 0.93)})`;
    screen.current.style.opacity = String(kit.remap(away, 0.25, 1.1, 1, 0.35));
  }, [still]);
  useOnScroll(recede);

  const toggleSound = () => {
    if (!video.current) return;
    video.current.muted = !muted;
    setMuted(!muted);
  };

  return (
    <section ref={root} className="relative mx-auto max-w-6xl px-4 pt-10 pb-24 sm:px-6 sm:pt-16">
      <p className="eyebrow" data-rise="0">
        {t.landing.hero.eyebrow}
      </p>
      <h1 className="display mt-5 max-w-5xl text-5xl text-balance leading-[1.02] font-semibold sm:text-7xl">
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
      <div className="mt-14 origin-top sm:mt-20" data-rise="1">
        <div ref={screen} className="screen relative aspect-video overflow-hidden bg-black">
          <video
            ref={video}
            className="size-full object-cover"
            src={film}
            poster={poster}
            aria-label={t.landing.hero.videoLabel}
            autoPlay={!still}
            controls={still}
            muted
            loop
            playsInline
          />
          {!still && (
            <button
              type="button"
              onClick={toggleSound}
              aria-label={muted ? t.landing.hero.soundOn : t.landing.hero.soundOff}
              className="absolute right-4 bottom-4 grid size-10 place-items-center rounded-full bg-black/50 text-[#f4efe6] backdrop-blur hover:bg-black/70"
            >
              {muted ? <VolumeX className="size-5" /> : <Volume2 className="size-5" />}
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
