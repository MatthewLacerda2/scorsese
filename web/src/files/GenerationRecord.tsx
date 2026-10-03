// How a generated file was made, and what it cost (#537): the brief, the
// model and its settings, when, and the charge — so a user who pays per
// generation can check it without asking the operator. The estimate is
// scorsese's own figure from its price table (docs/prices.md); the charge is
// what the ledger took, cost plus 10%.

import type { GenerationRecord as Record } from "@/api";
import { useBalance } from "@/app/queries";
import { formatDate } from "@/lib/format";
import { formatDollars, formatMoney, toCentavos } from "@/lib/money";

const TITLE = {
  veo_shot: "Generated video",
  still_image: "Generated still",
  spoken_line: "Generated speech",
  voice_design: "Voice design sample",
};

export function GenerationRecord({ record }: { record: Record }) {
  const rate = useBalance().data?.rate ?? null;
  const money = (micros: number) =>
    formatMoney(micros, rate === null ? null : toCentavos(micros, rate.brl_per_usd_e4));

  return (
    <section className="flex flex-col gap-2 rounded-lg border p-3">
      <h3 className="text-sm font-medium">{TITLE[record.kind]}</h3>
      <p className="text-sm whitespace-pre-wrap">
        {record.kind === "spoken_line" ? record.text : record.prompt}
      </p>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">Model</dt>
        <dd>{record.model}</dd>
        {settings(record).map(([name, value]) => (
          <Row key={name} name={name} value={value} />
        ))}
        <dt className="text-muted-foreground">Made</dt>
        <dd>{formatDate(record.created_at)}</dd>
        <dt className="text-muted-foreground">Cost</dt>
        <dd>
          {record.charged_micros > 0 ? (
            <>
              {money(record.charged_micros)}{" "}
              <span className="text-muted-foreground">
                ({formatDollars(record.charged_micros)})
              </span>
            </>
          ) : (
            "Nothing was charged"
          )}
        </dd>
        <dt className="text-muted-foreground">Estimate</dt>
        <dd className="text-muted-foreground">
          {formatDollars(record.estimated_cost_micros)} at the provider's listed price
        </dd>
      </dl>
    </section>
  );
}

function Row({ name, value }: { name: string; value: string }) {
  return (
    <>
      <dt className="text-muted-foreground">{name}</dt>
      <dd>{value}</dd>
    </>
  );
}

/** The settings a brief was made with, as label/value pairs. */
function settings(record: Record): [string, string][] {
  if (record.kind !== "spoken_line") {
    const pairs: [string, string | number | undefined][] = [
      ["Resolution", record.resolution],
      ["Length", record.seconds === undefined ? undefined : `${record.seconds} s`],
      ["Aspect", record.aspect],
      ["References", record.references || undefined],
      ["Seed", record.seed ?? undefined],
    ];
    return pairs.flatMap(([name, value]) => (value === undefined ? [] : [[name, String(value)]]));
  }
  const pairs: [string, string][] = record.voice ? [["Voice", record.voice]] : [];
  for (const [name, value] of Object.entries(record.settings ?? {})) {
    pairs.push([name.replaceAll("_", " "), String(value)]);
  }
  return pairs;
}
