// One card in a landing section's row: an icon, a title, a line, and an
// optional step number, rising in after the one before it. The icon sits on
// top so a narrow card (five in a row) still gives its title the full width.

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
    <div className="card flex flex-col p-6 sm:p-7" data-rise={at(props.index)}>
      <div className="flex items-start justify-between">
        <span className="grid size-11 place-items-center rounded-xl bg-gold/15">
          <Icon className="gold size-5" strokeWidth={1.75} aria-hidden />
        </span>
        {props.step && (
          <span className="dim text-sm font-medium tabular-nums" aria-hidden>
            {props.step}
          </span>
        )}
      </div>
      <h3 className="display mt-5 text-xl leading-snug font-medium">{props.title}</h3>
      <p className="dim mt-2 leading-relaxed">{props.body}</p>
    </div>
  );
}
