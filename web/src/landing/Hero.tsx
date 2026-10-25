// The first screen of the landing page: scorsese's mark and name, what to do
// ("Describe your video") and the prompt box, centred and alone. Everything
// else the page says is below it. The header above (`Header.tsx`) carries only
// the language and the way in. The mark is the app's 512px icon
// (app/assets/icon.png) as a white silhouette with no background, made once:
//
//   ffmpeg -i app/assets/icon.png -filter_complex "[0]format=gray[a];
//     color=white:s=512x512,format=rgba[w];[w][a]alphamerge,format=rgba"
//     -frames:v 1 web/src/landing/media/mark.png
//
// Its brightness becomes its alpha. `mark-light.png` is the same in black
// (app/assets/README.md); `Mark` shows the one for the theme.

import { useRef } from "react";
import { Mark } from "@/components/Mark";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { useEntrance } from "@/landing/motion";
import { PromptBox } from "@/prompt/PromptBox";

export function Hero({ onSignIn }: { onSignIn: () => void }) {
  const t = useT();
  const { language } = useLanguage();
  const root = useRef<HTMLElement>(null);
  useEntrance(root, language);

  return (
    <section
      ref={root}
      className="mx-auto flex min-h-svh max-w-2xl flex-col items-center justify-center px-4 pt-16 pb-16 text-center sm:px-6"
    >
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
        className="display mt-8 text-2xl font-medium text-balance sm:mt-10 sm:text-3xl"
        data-rise="0.6"
      >
        {t.landing.hero.title}
      </h1>
      <div className="mt-6 w-full" data-rise="0.8">
        <PromptBox onSubmit={onSignIn} />
      </div>
    </section>
  );
}
