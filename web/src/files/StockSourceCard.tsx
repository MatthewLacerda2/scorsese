// Where a stock file came from (#1237): the library, who published it, its
// licence and a link to its page — recorded when the assistant imported it,
// and shown where a generated file shows how it was made.

import type { StockSource } from "@/api";
import { useT } from "@/i18n/I18nProvider";

export function StockSourceCard({ source }: { source: StockSource }) {
  const words = useT().files.source;
  const library = words.libraries[source.library];
  return (
    <section className="flex flex-col gap-2 rounded-lg border p-3">
      <h3 className="text-sm font-medium">{words.title(library)}</h3>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        {source.author && (
          <>
            <dt className="text-muted-foreground">{words.author}</dt>
            <dd>{source.author}</dd>
          </>
        )}
        {source.licence && (
          <>
            <dt className="text-muted-foreground">{words.licence}</dt>
            <dd>{source.licence}</dd>
          </>
        )}
      </dl>
      {source.page_url && (
        <a
          href={source.page_url}
          target="_blank"
          rel="noopener noreferrer"
          className="text-sm underline"
        >
          {words.page(library)}
        </a>
      )}
    </section>
  );
}
