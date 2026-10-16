// One card in a landing section's row: an icon, a title, a line, and an
// optional step number, rising in after the one before it.

import type { LucideIcon } from "lucide-react";
import { kit } from "@/landing/kit";

/** When the `index`th item of a row rises in: one after another, from 0.2. */
const at = (index: number) => String(kit.stagger(index, 0.06, 0.2));

export function Feature(props: {
  icon: LucideIcon;
  title: string;
  body: string;
  index: number;
  step?: string;
}) {
  const Icon = props.icon;
  return (
    <div className="card p-6 sm:p-8" data-rise={at(props.index)}>
      <div className="flex items-center gap-3">
        <Icon className="gold size-7 shrink-0" strokeWidth={1.5} />
        <h3 className="display text-2xl leading-tight font-medium">{props.title}</h3>
        {props.step && <span className="mono dim ml-auto text-sm">{props.step}</span>}
      </div>
      <p className="dim mt-4 leading-relaxed">{props.body}</p>
    </div>
  );
}
