import { useT } from "@/i18n/I18nProvider";
import { QueryError } from "./query-error";

/**
 * What a list page says while its request has not produced a list: why it
 * failed, that it is loading, or — when the page finds the list empty and
 * passes `empty` — what an empty one means. The list itself stays the page's.
 */
export function QueryState({
  query,
  empty = null,
}: {
  query: { isPending: boolean; isError: boolean; error: Error | null };
  /** The words for an empty list, or nothing while the list has something. */
  empty?: string | null;
}) {
  const t = useT();
  return (
    <>
      {query.isError && <QueryError error={query.error} className="text-destructive" />}
      {query.isPending && <p className="text-muted-foreground">{t.common.loading}</p>}
      {empty && <p className="text-muted-foreground">{empty}</p>}
    </>
  );
}
