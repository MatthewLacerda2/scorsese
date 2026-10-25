// The frame of a landing section below the first screen: an eyebrow, a title
// and what the section shows. Each moves in as it is scrolled to
// (`ScrollIn.tsx`): what carries `data-rise` rises in, staggered by
// `kit.stagger`.

import type { ReactNode } from "react";

/** A section that moves with the scroll, under an eyebrow and a title. */
export function Section(props: {
  id?: string;
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section id={props.id} className="mx-auto max-w-6xl scroll-mt-20 px-4 py-20 sm:px-6 sm:py-28">
      <p className="eyebrow" data-rise="0.05">
        {props.eyebrow}
      </p>
      <h2
        className="display mt-4 max-w-3xl text-4xl leading-tight font-medium text-balance sm:text-5xl"
        data-rise="0.1"
      >
        {props.title}
      </h2>
      {props.children}
    </section>
  );
}
