// The first screen of the landing page: scorsese's mark and name, the heading,
// the prompt box and the line saying the start is free, with the strip of
// films made with scorsese along the bottom edge, so what comes out is on the
// first screen too. Everything else the page says is below it. On a wide
// screen the words and the box are centred; on a phone the mark and the name
// are smaller and the rest sits high (a third of the room to spare above it,
// two below), so all of it shows at 360×640 and the box stays above the
// keyboard as it opens (#1566), while the strip keeps its place at the bottom. The mark is the app's icon as a silhouette with no
// background, in the theme's ink (`Mark`; app/assets/README.md has how each
// copy is made).

import { useRef } from "react";
import { Mark } from "@/components/Mark";
import { useT } from "@/i18n/I18nProvider";
import { Carousel } from "@/landing/Carousel";
import { FreeLine } from "@/landing/FreeLine";
import { useEntrance } from "@/landing/motion";
import { PromptBox, type PromptSubmit } from "@/prompt/PromptBox";

export function Hero({ onPrompt }: { onPrompt: PromptSubmit }) {
  const t = useT();
  const root = useRef<HTMLElement>(null);
  useEntrance(root);

  return (
    <section ref={root} className="flex min-h-svh flex-col pt-20 pb-8 max-sm:pt-[4.5rem] sm:pb-10">
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col items-center px-4 pb-12 text-center before:grow before:content-[''] after:grow-[2] after:content-[''] sm:justify-center sm:px-6 sm:before:hidden sm:after:hidden">
        {/* The mark and the name come up from behind an edge, not fading in (`data-emerge`). */}
        <span className="block overflow-hidden">
          <Mark
            kind="silhouette"
            size={86}
            className="size-11 sm:size-[5.4rem]"
            data-emerge="0.2"
          />
        </span>
        <span className="block overflow-hidden pb-2">
          <span className="display block text-4xl font-semibold sm:text-7xl" data-emerge="0.35">
            scorsese
          </span>
        </span>
        <h1
          className="display mt-4 text-2xl font-medium text-balance sm:mt-9 sm:text-3xl"
          data-rise="0.6"
        >
          {t.landing.hero.title}
        </h1>
        <div className="mt-5 w-full sm:mt-7" data-rise="0.8">
          <PromptBox onSubmit={onPrompt} />
          <FreeLine />
        </div>
      </div>
      <div data-rise="1.1">
        <Carousel label={t.landing.showcase.label} />
      </div>
    </section>
  );
}
