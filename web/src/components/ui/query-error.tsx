import { cn } from "@/lib/utils";

/**
 * The words of a failed request, in the destructive colour — nothing while
 * there is no error. Pass a request's `error` as it comes (`null` until it
 * fails); `className` replaces the default size and `as` the element, for the
 * places that sit inside a line of text.
 */
export function QueryError({
  error,
  className = "text-sm text-destructive",
  as: Element = "p",
}: {
  error: Error | null | undefined;
  className?: string;
  as?: "p" | "span";
}) {
  if (!error) return null;
  return <Element className={cn(className)}>{error.message}</Element>;
}
