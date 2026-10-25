// The strip of films under the landing page's headline: short cuts of videos
// made with scorsese, playing as they slide past. They are decoration, not
// players: muted, looping, with no controls, no fullscreen and nothing to
// click, so a visitor's eye is on the headline and the strip only says what
// comes out. Each cut is its speed already (2x for the longer ones), baked in
// when it was cut, so nothing here sets a playback rate.
//
// The strip is the clips three times over and slides one copy's width, then
// starts again, which loops without a seam on any screen up to three copies
// wide. A visitor who asked for less motion gets the stills, standing. Until
// the strip is near the screen only its stills are fetched (`preload="none"`,
// no autoplay, #1566), so a phone that never scrolls to it never downloads a
// film; near, its films play, and scrolled out of view they pause, so a phone
// is not decoding twelve videos nobody sees.

import { type RefObject, useEffect, useRef } from "react";
import cyberpunkStill from "@/landing/media/showcase/cyberpunk.jpg";
import cyberpunk from "@/landing/media/showcase/cyberpunk.mp4";
import hivemindStill from "@/landing/media/showcase/hivemind.jpg";
import hivemind from "@/landing/media/showcase/hivemind.mp4";
import summitStill from "@/landing/media/showcase/summit.jpg";
import summit from "@/landing/media/showcase/summit.mp4";
import trmStill from "@/landing/media/showcase/trm.jpg";
import trm from "@/landing/media/showcase/trm.mp4";
import { prefersStill } from "@/lib/reducedMotion";

const CLIPS = [
  { film: hivemind, still: hivemindStill },
  { film: summit, still: summitStill },
  { film: trm, still: trmStill },
  { film: cyberpunk, still: cyberpunkStill },
];
const COPIES = 3;

/** How far off the screen the strip counts as near: its films start loading there. */
const NEAR = "300px 0px";

/** Plays the films under `ref` while it is near the screen, and pauses them once it is not. */
function usePlayWhileSeen(ref: RefObject<HTMLElement | null>, still: boolean) {
  useEffect(() => {
    const root = ref.current;
    if (!root || still) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        for (const film of root.querySelectorAll("video")) {
          if (entry?.isIntersecting) void film.play()?.catch(() => {});
          else film.pause();
        }
      },
      { rootMargin: NEAR },
    );
    observer.observe(root);
    return () => observer.disconnect();
  }, [ref, still]);
}

export function Carousel({ label }: { label: string }) {
  const still = prefersStill();
  const root = useRef<HTMLDivElement>(null);
  usePlayWhileSeen(root, still);
  const strip = Array.from({ length: COPIES }, (_, copy) =>
    CLIPS.map((clip) => ({ ...clip, copy })),
  ).flat();
  return (
    <div ref={root} className="carousel" role="img" aria-label={label}>
      <div className="carousel-strip">
        {strip.map((clip) => (
          <video
            key={`${clip.copy}-${clip.film}`}
            className="carousel-clip"
            src={clip.film}
            poster={clip.still}
            preload="none"
            muted
            loop
            playsInline
            disablePictureInPicture
            disableRemotePlayback
            tabIndex={-1}
            aria-hidden
          />
        ))}
      </div>
    </div>
  );
}
