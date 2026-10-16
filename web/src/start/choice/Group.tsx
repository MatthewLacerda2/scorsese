// The two choices a project is started for (#1016) are each a radio group whose
// first option is "none", because both are optional and a person with a clear
// idea skips them. This is the group; `Option.tsx` is one of its options.

import type { ReactNode } from "react";

/** A choice's heading, the line saying it can change later, and its options. */
export function Group({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: ReactNode;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">{hint}</p>
      {children}
    </div>
  );
}
