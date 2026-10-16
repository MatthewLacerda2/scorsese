// The landing page below the top: how it works, what it handles, and a last
// way in. Each is a slide (`Slides.tsx`): what carries `data-rise` rises in as
// the slide fades in, staggered by `kit.stagger`.

import type { ReactNode } from "react";

/** A section that moves with the scroll, under an eyebrow and a title. */
export function Section(props: {
  id?: string;
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section id={props.id} className="mx-auto max-w-6xl scroll-mt-8 px-4 py-20 sm:px-6 sm:py-28">
      <p className="eyebrow" data-rise="0.05">
        {props.eyebrow}
      </p>
      <h2
        className="display mt-4 max-w-3xl text-4xl leading-tight font-medium sm:text-5xl"
        data-rise="0.1"
      >
        {props.title}
      </h2>
      {props.children}
    </section>
  );
}
