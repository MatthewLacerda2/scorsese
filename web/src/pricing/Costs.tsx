// "What things cost": real examples at what the user pays, from
// `pricing/examples.ts`, and one line on what a day's free credit covers.

import { useT } from "@/i18n/I18nProvider";
import { formatDollars } from "@/lib/money";
import { COST_EXAMPLES } from "@/pricing/examples";

export function Costs() {
  const t = useT().pricing.costs;
  return (
    <section className="mt-20">
      <h2 className="display text-3xl font-medium sm:text-4xl">{t.title}</h2>
      <p className="dim mt-3">{t.lead}</p>
      <dl className="mt-8 divide-y divide-paper/10 border-y border-paper/10">
        {COST_EXAMPLES.map((example) => {
          const amount = example.toMicros
            ? `${formatDollars(example.fromMicros)}–${formatDollars(example.toMicros)}`
            : formatDollars(example.fromMicros);
          return (
            <div key={example.id} className="flex items-baseline justify-between gap-6 py-4">
              <dt>{t[example.id]}</dt>
              <dd className="display shrink-0 text-lg font-medium">{t.around(amount)}</dd>
            </div>
          );
        })}
      </dl>
      <p className="mt-6 text-lg">{t.footnote}</p>
    </section>
  );
}
