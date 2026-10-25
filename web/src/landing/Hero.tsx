// The first screen of the landing page: scorsese's mark and name, the heading
// and the prompt box, centred, with the strip of films made with scorsese
// along the bottom edge, so what comes out is on the first screen too.
// Everything else the page says is below it. The mark is the app's icon as a
// silhouette with no background, in the theme's ink (`Mark`; app/assets/README.md
// has how each copy is made).

import { useRef } from "react";
import { Mark } from "@/components/Mark";
import { useT } from "@/i18n/I18nProvider";
import { Carousel } from "@/landing/Carousel";
import { useEntrance } from "@/landing/motion";
import { PromptBox } from "@/prompt/PromptBox";

export function Hero({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  const root = useRef<HTMLElement>(null);
  useEntrance(root);

  return (
    <section ref={root} className="flex min-h-svh flex-col pt-20 pb-8 sm:pb-10">
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col items-center justify-center px-4 pb-12 text-center sm:px-6">
        {/* The mark and the name come up from behind an edge, not fading in (`data-emerge`). */}
        <span className="block overflow-hidden">
          <Mark
            kind="silhouette"
            size={86}
            className="size-[4.2rem] sm:size-[5.4rem]"
            data-emerge="0.2"
          />
        </span>
        <span className="block overflow-hidden pb-2">
          <span className="display block text-5xl font-semibold sm:text-7xl" data-emerge="0.35">
            scorsese
          </span>
        </span>
        <h1
          className="display mt-7 text-2xl font-medium text-balance sm:mt-9 sm:text-3xl"
          data-rise="0.6"
        >
          {t.landing.hero.title}
        </h1>
        <div className="mt-7 w-full" data-rise="0.8">
          <PromptBox onSubmit={onSignIn} />
        </div>
      </div>
      <div data-rise="1.1">
        <Carousel label={t.landing.showcase.label} />
      </div>
    </section>
  );
}
