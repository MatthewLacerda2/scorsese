// The strip of films under the landing page's headline: short cuts of videos
// made with scorsese, playing as they slide past. They are decoration, not
// players: muted, looping, with no controls, no fullscreen and nothing to
// click, so a visitor's eye is on the headline and the strip only says what
// comes out. Each cut is its speed already (2x for the longer ones), baked in
// when it was cut, so nothing here sets a playback rate.
//
// The strip is the clips three times over and slides one copy's width, then
// starts again, which loops without a seam on any screen up to three copies
// wide. A visitor who asked for less motion gets the stills, standing.

import cyberpunkStill from "@/landing/media/showcase/cyberpunk.jpg";
import cyberpunk from "@/landing/media/showcase/cyberpunk.mp4";
import hivemindStill from "@/landing/media/showcase/hivemind.jpg";
import hivemind from "@/landing/media/showcase/hivemind.mp4";
import summitStill from "@/landing/media/showcase/summit.jpg";
import summit from "@/landing/media/showcase/summit.mp4";
import trmStill from "@/landing/media/showcase/trm.jpg";
import trm from "@/landing/media/showcase/trm.mp4";
import { prefersStill } from "@/landing/motion";

const CLIPS = [
  { film: hivemind, still: hivemindStill },
  { film: summit, still: summitStill },
  { film: trm, still: trmStill },
  { film: cyberpunk, still: cyberpunkStill },
];
const COPIES = 3;

export function Carousel({ label }: { label: string }) {
  const still = prefersStill();
  const strip = Array.from({ length: COPIES }, (_, copy) =>
    CLIPS.map((clip) => ({ ...clip, copy })),
  ).flat();
  return (
    <div className="carousel" role="img" aria-label={label}>
      <div className="carousel-strip">
        {strip.map((clip) => (
          <video
            key={`${clip.copy}-${clip.film}`}
            className="carousel-clip"
            src={clip.film}
            poster={clip.still}
            autoPlay={!still}
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
