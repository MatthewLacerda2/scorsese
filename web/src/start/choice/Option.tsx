// One option of a picker's radio group (`Group.tsx`): a card, which holds
// whatever the picker draws in it.

import { cn } from "cn";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";

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
    <Button
      type="button"
      variant="outline"
      role="radio"
      aria-checked={checked}
      onClick={onPick}
      className={cn(
        "h-auto flex-col items-start gap-1 whitespace-normal p-2 text-left font-normal",
        checked && "border-primary ring-2 ring-primary/30",
      )}
    >
      {children}
    </Button>
  );
}
