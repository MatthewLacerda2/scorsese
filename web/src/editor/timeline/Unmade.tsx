// What a clip nobody has made yet carries under its label: its kind's hue as a
// pale tint, hatching in the same hue, and how far along its brief is — the
// desktop app's `paint/clip.rs`, in CSS. The hatching says *this is planned*;
// pale enough that the clip's own name still reads on top of it.

import { kindColor } from "../assets/kinds";

/** Diagonal stripes, as a mask over the kind's colour: 2px of hue every 9px,
 * the app's spacing. A mask rather than a gradient so the hue stays the one
 * Tailwind class `kindColor` names, shared with the pool and the inspector. */
const HATCH = "repeating-linear-gradient(135deg, transparent 0 7px, black 7px 9px)";

export function Unmade(props: { kind: string | undefined; state: string | undefined }) {
  const hue = kindColor(props.kind);
  return (
    <>
      <span className={`pointer-events-none absolute inset-0 opacity-15 ${hue}`} aria-hidden />
      <span
        className={`pointer-events-none absolute inset-0 opacity-45 ${hue}`}
        style={{ maskImage: HATCH, WebkitMaskImage: HATCH }}
        aria-hidden
      />
      <span
        className="pointer-events-none absolute inset-0 rounded-sm ring-1 ring-foreground/20 ring-inset"
        aria-hidden
      />
      {/* Only on what does not exist yet: "generated" on a clip that plays is
          a word taking up room to say everything is normal. */}
      {props.state && (
        <span className="pointer-events-none absolute top-0 right-2 leading-10 text-[10px] text-muted-foreground">
          {props.state}
        </span>
      )}
    </>
  );
}
