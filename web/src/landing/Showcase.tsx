// One whole video made with scorsese, in the visitor's language: the vertical
// post made to announce scorsese itself, cut in Portuguese and in English.
// Portuguese visitors see theirs; everyone else sees the English one. Like the
// strip above it (`Carousel`), it is a picture, not a player: muted, looping,
// nothing to click.

import { useLanguage, useT } from "@/i18n/I18nProvider";
import englishStill from "@/landing/media/showcase/linkedin-en.jpg";
import english from "@/landing/media/showcase/linkedin-en.mp4";
import portugueseStill from "@/landing/media/showcase/linkedin-pt.jpg";
import portuguese from "@/landing/media/showcase/linkedin-pt.mp4";
import { prefersStill } from "@/landing/motion";

export function Showcase() {
  const t = useT().landing.showcase;
  const { language } = useLanguage();
  const [film, still] =
    language === "pt-BR" ? [portuguese, portugueseStill] : [english, englishStill];
  return (
    <section className="mx-auto max-w-6xl px-4 py-16 text-center sm:px-6 sm:py-20">
      <p className="eyebrow" data-rise="0.05">
        {t.eyebrow}
      </p>
      <div data-rise="0.15">
        <video
          key={film}
          className="screen pointer-events-none mx-auto mt-8 aspect-[4/5] h-[min(72svh,46rem)] max-w-full bg-black object-cover"
          src={film}
          poster={still}
          autoPlay={!prefersStill()}
          muted
          loop
          playsInline
          disablePictureInPicture
          disableRemotePlayback
          tabIndex={-1}
          aria-label={t.label}
        />
      </div>
    </section>
  );
}
