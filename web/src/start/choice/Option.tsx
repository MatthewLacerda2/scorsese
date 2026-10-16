// One option of a picker's radio group (`Group.tsx`): a card, which holds
// whatever the picker draws in it.

import { cn } from "cn";
import type { ReactNode } from "react";

/** A card that is checked when chosen. */
export function Option({
  checked,
  onPick,
  children,
}: {
  checked: boolean;
  onPick: () => void;
  children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a card with a picture in it, not a bare radio input.
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      onClick={onPick}
      className={cn(
        "flex flex-col items-start gap-1 rounded-lg border p-2 text-left text-sm transition-colors hover:bg-muted/50",
        checked && "border-primary ring-2 ring-primary/30",
      )}
    >
      {children}
    </button>
  );
}
