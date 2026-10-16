// One whole video made with scorsese, in the visitor's language: the promo
// made to announce scorsese itself, cut at 16:9 for this page (the LinkedIn
// post re-laid out wide, its end card pointing here), in Portuguese and in
// English, whole and at its own speed. Portuguese visitors see theirs;
// everyone else sees the English one.
//
// It starts muted, since a browser plays nothing aloud on its own, and loops.
// It starts when the picture is first seen, not at page load below the fold,
// and pauses when the visitor stops seeing it: the section scrolled mostly out
// of view, or another tab in front (#1310). Coming back does not resume it; the
// play button, or a click on the picture, does. Its other controls are the
// sound: a button to turn it on or off and a volume slider, the slider left off
// on a phone, where the video is short enough for it to cover the picture and
// the phone's own buttons set the volume. No fullscreen, no seeking, nothing
// else to learn.

import { Pause, Play, Volume2, VolumeX } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import englishStill from "@/landing/media/showcase/promo-en.jpg";
import english from "@/landing/media/showcase/promo-en.mp4";
import portugueseStill from "@/landing/media/showcase/promo-pt.jpg";
import portuguese from "@/landing/media/showcase/promo-pt.mp4";
import { prefersStill } from "@/landing/motion";

/** How much of the picture must show for the visitor to count as watching. */
const MOSTLY = 0.5;

export function Showcase() {
  const t = useT().landing.showcase;
  const { language } = useLanguage();
  const video = useRef<HTMLVideoElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const [playing, setPlaying] = useState(false);
  const [muted, setMuted] = useState(true);
  const [volume, setVolume] = useState(0.8);
  const [film, still] =
    language === "pt-BR" ? [portuguese, portugueseStill] : [english, englishStill];

  useEffect(() => {
    const element = frame.current;
    if (!element) return;
    let seen = false;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry) return;
        if (entry.intersectionRatio < MOSTLY) video.current?.pause();
        else if (!seen) {
          seen = true;
          if (!prefersStill()) void video.current?.play()?.catch(() => {});
        }
      },
      { threshold: [0, MOSTLY] },
    );
    observer.observe(element);
    const away = () => {
      if (document.visibilityState === "hidden") video.current?.pause();
    };
    document.addEventListener("visibilitychange", away);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", away);
    };
  }, []);

  const playPause = () => {
    const player = video.current;
    if (!player) return;
    if (player.paused) void player.play()?.catch(() => {});
    else player.pause();
  };
  const toggle = () => {
    const player = video.current;
    if (!player) return;
    player.muted = !muted;
    player.volume = volume;
    // A visitor who asked for less motion starts it here.
    if (player.paused) void player.play()?.catch(() => {});
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
        ref={frame}
        className="screen relative mx-auto mt-8 aspect-video w-[min(100%,128svh,64rem)] overflow-hidden bg-screen"
        data-rise="0.15"
      >
        <video
          key={film}
          ref={video}
          className="size-full object-contain"
          src={film}
          poster={still}
          muted={muted}
          loop
          playsInline
          disablePictureInPicture
          disableRemotePlayback
          aria-label={t.label}
          onClick={playPause}
          onPlay={() => setPlaying(true)}
          onPause={() => setPlaying(false)}
        />
        <div className="absolute right-3 bottom-3 flex items-center gap-2 rounded-full bg-screen/55 p-1.5 text-paper sm:pr-4 backdrop-blur">
          <button
            type="button"
            onClick={playPause}
            aria-label={playing ? t.pause : t.play}
            className="grid size-9 place-items-center rounded-full hover:bg-on-screen/10"
          >
            {playing ? <Pause className="size-5" /> : <Play className="size-5" />}
          </button>
          <button
            type="button"
            onClick={toggle}
            aria-label={muted ? t.soundOn : t.soundOff}
            className="grid size-9 place-items-center rounded-full hover:bg-on-screen/10"
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
