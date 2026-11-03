// The first screen of the landing page: scorsese's mark, the headline saying
// what to do now, a muted line under it with what the site does for you, the
// prompt box and the line saying the start is free, with the strip of films
// made with scorsese along the bottom edge, so what comes out is on the first
// screen too (#1593); the `hero-strip` test's version `b` (#1555) starts the
// strip just below the first screen instead. Everything else the page says is
// below it. On a wide screen the words and the box are centred; on a phone the
// mark is smaller and the rest sits high (a third of the room to spare above
// it, two below), so all of it shows at 360×640 and the box stays above the
// keyboard as it opens (#1566), while the strip keeps its place at the bottom.
// The mark is the app's icon as a silhouette with no background, in the
// theme's ink (`Mark`; app/assets/README.md has how each copy is made).

import { useRef } from "react";
import { Mark } from "@/components/Mark";
import { useT } from "@/i18n/I18nProvider";
import { Carousel } from "@/landing/Carousel";
import { FreeLine } from "@/landing/FreeLine";
import { useEntrance } from "@/landing/motion";
import { PromptBox, type PromptSubmit } from "@/prompt/PromptBox";

/**
 * `version` is the `hero-subtitle` test's (#1593): `b` says no editing is
 * needed, anything else is today's line. `layout` is the `hero-strip` test's
 * (#1555): `b` keeps the first screen to the prompt box alone and starts the
 * strip below it, anything else is today's screen with the strip on it.
 */
export function Hero({
  onPrompt,
  version,
  layout,
}: {
  onPrompt: PromptSubmit;
  version?: string;
  layout?: string;
}) {
  const t = useT();
  const root = useRef<HTMLElement>(null);
  useEntrance(root);

  const words = (
    <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col items-center px-4 pb-12 text-center before:grow before:content-[''] after:grow-[2] after:content-[''] sm:justify-center sm:px-6 sm:before:hidden sm:after:hidden">
      {/* The mark and the headline come up from behind an edge, not fading in (`data-emerge`). */}
      <span className="block overflow-hidden">
        <Mark kind="silhouette" size={72} className="size-11 sm:size-[4.5rem]" data-emerge="0.2" />
      </span>
      <div className="mt-4 overflow-hidden pb-1 sm:mt-6">
        <h1
          className="display block text-[2.125rem] leading-[1.1] text-balance sm:text-6xl"
          data-emerge="0.35"
        >
          {t.landing.hero.title}
        </h1>
      </div>
      <p
        className="dim mt-3 text-base text-balance sm:mt-5 sm:text-xl"
        data-rise="0.6"
        data-subtitle
      >
        {version === "b" ? t.landing.hero.subtitleB : t.landing.hero.subtitle}
      </p>
      <div className="mt-6 w-full sm:mt-9" data-rise="0.8">
        <PromptBox onSubmit={onPrompt} />
        <FreeLine />
      </div>
    </div>
  );
  const strip = (
    <div data-rise="1.1">
      <Carousel label={t.landing.showcase.label} />
    </div>
  );
  // The first screen (`data-first-screen`) is the window's full height either way.
  const screen = "flex min-h-svh flex-col pt-20 max-sm:pt-[4.5rem]";

  if (layout === "b")
    return (
      <section ref={root} className="pb-8 sm:pb-10">
        <div className={screen} data-first-screen>
          {words}
        </div>
        {strip}
      </section>
    );
  return (
    <section ref={root} className={`${screen} pb-8 sm:pb-10`} data-first-screen>
      {words}
      {strip}
    </section>
  );
}
