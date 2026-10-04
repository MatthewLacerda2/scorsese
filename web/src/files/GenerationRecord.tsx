// How a generated file was made, and what it cost (#537): the brief, the
// model and its settings, when, and the charge — so a user who pays per
// generation can check it without asking the operator. The estimate is
// scorsese's own figure from its price table (docs/prices.md); the charge is
// what the ledger took, cost plus 10%.

import type { GenerationRecord as Record } from "@/api";
import type { Messages } from "@/i18n/catalogue";
import { useLanguage, useT } from "@/i18n/I18nProvider";
import { formatDate } from "@/lib/format";
import { formatDollars } from "@/lib/money";

export function GenerationRecord({ record }: { record: Record }) {
  const t = useT();
  const { language } = useLanguage();
  const words = t.files.generation;
  return (
    <section className="flex flex-col gap-2 rounded-lg border p-3">
      <h3 className="text-sm font-medium">{words.title[record.kind]}</h3>
      <p className="text-sm whitespace-pre-wrap">
        {record.kind === "spoken_line" ? record.text : record.prompt}
      </p>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{words.model}</dt>
        <dd>{record.model}</dd>
        {settings(record, words, language).map(([name, value]) => (
          <Row key={name} name={name} value={value} />
        ))}
        <dt className="text-muted-foreground">{words.made}</dt>
        <dd>{formatDate(record.created_at, language)}</dd>
        <dt className="text-muted-foreground">{words.cost}</dt>
        <dd>{record.charged_micros > 0 ? formatDollars(record.charged_micros) : words.free}</dd>
        <dt className="text-muted-foreground">{words.estimate}</dt>
        <dd className="text-muted-foreground">
          {words.listedPrice(formatDollars(record.estimated_cost_micros))}
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

/**
 * The settings a brief was made with, as label/value pairs. A spoken line's
 * own settings are named by the server (`stability`, `similarity boost`) and
 * keep those names.
 */
function settings(
  record: Record,
  words: Messages["files"]["generation"],
  language: string,
): [string, string][] {
  if (record.kind !== "spoken_line") {
    const pairs: [string, string | number | undefined][] = [
      [words.resolution, record.resolution],
      [
        words.length,
        record.seconds === undefined
          ? undefined
          : words.seconds(new Intl.NumberFormat(language).format(record.seconds)),
      ],
      [words.aspect, record.aspect],
      [words.references, record.references || undefined],
      [words.seed, record.seed ?? undefined],
    ];
    return pairs.flatMap(([name, value]) => (value === undefined ? [] : [[name, String(value)]]));
  }
  const pairs: [string, string][] = record.voice ? [[words.voice, record.voice]] : [];
  for (const [name, value] of Object.entries(record.settings ?? {})) {
    pairs.push([name.replaceAll("_", " "), String(value)]);
  }
  return pairs;
}
