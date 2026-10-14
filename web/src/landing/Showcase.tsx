// One whole video made with scorsese, in the visitor's language: the promo
// made to announce scorsese itself, cut at 16:9 for this page (the LinkedIn
// post re-laid out wide, its end card pointing here), in Portuguese and in
// English, whole and at its own speed. Portuguese visitors see theirs;
// everyone else sees the English one.
//
// It starts muted, since a browser plays nothing aloud on its own, and loops.
// Its only controls are the sound: a button to turn it on or off and a volume
// slider, the slider left off on a phone, where the video is short enough for
// it to cover the picture and the phone's own buttons set the volume. No
// fullscreen, no seeking, nothing else to learn.

import { Volume2, VolumeX } from "lucide-react";
import { useRef, useState } from "react";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import englishStill from "@/landing/media/showcase/promo-en.jpg";
import english from "@/landing/media/showcase/promo-en.mp4";
import portugueseStill from "@/landing/media/showcase/promo-pt.jpg";
import portuguese from "@/landing/media/showcase/promo-pt.mp4";
import { prefersStill } from "@/landing/motion";

export function Showcase() {
  const t = useT().landing.showcase;
  const { language } = useLanguage();
  const video = useRef<HTMLVideoElement>(null);
  const [muted, setMuted] = useState(true);
  const [volume, setVolume] = useState(0.8);
  const [film, still] =
    language === "pt-BR" ? [portuguese, portugueseStill] : [english, englishStill];

  const toggle = () => {
    const player = video.current;
    if (!player) return;
    player.muted = !muted;
    player.volume = volume;
    // A visitor who asked for less motion starts it here.
    if (player.paused) void player.play();
    setMuted(!muted);
  };
  const changeVolume = (next: number) => {
    setVolume(next);
    const player = video.current;
    if (!player) return;
    player.volume = next;
    player.muted = next === 0;
    setMuted(next === 0);
  };

  return (
    <section className="mx-auto max-w-6xl px-4 py-16 text-center sm:px-6 sm:py-20">
      <p className="eyebrow" data-rise="0.05">
        {t.eyebrow}
      </p>
      <div
        className="screen relative mx-auto mt-8 aspect-video w-[min(100%,128svh,64rem)] overflow-hidden bg-black"
        data-rise="0.15"
      >
        <video
          key={film}
          ref={video}
          className="size-full object-contain"
          src={film}
          poster={still}
          autoPlay={!prefersStill()}
          muted={muted}
          loop
          playsInline
          disablePictureInPicture
          disableRemotePlayback
          aria-label={t.label}
        />
        <div className="absolute right-3 bottom-3 flex items-center gap-2 rounded-full bg-black/55 p-1.5 text-[#f4efe6] sm:pr-4 backdrop-blur">
          <button
            type="button"
            onClick={toggle}
            aria-label={muted ? t.soundOn : t.soundOff}
            className="grid size-9 place-items-center rounded-full hover:bg-white/10"
          >
            {muted ? <VolumeX className="size-5" /> : <Volume2 className="size-5" />}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={muted ? 0 : volume}
            onChange={(event) => changeVolume(Number(event.target.value))}
            aria-label={t.volume}
            className="volume hidden w-24 sm:block"
          />
        </div>
      </div>
    </section>
  );
}
