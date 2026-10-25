// "Add credits" (#548) can be asked for from the header's balance popup and
// from the lines that say a balance is empty (#1244). One dialog serves them
// all: the shell mounts it, and any component under it calls `useAddCredits()`
// to open it. The dialog is a chunk of its own, fetched on the first open.

import { createContext, lazy, type ReactNode, Suspense, useContext, useState } from "react";

const AddCreditsDialog = lazy(() =>
  import("@/payments/AddCredits").then((module) => ({ default: module.AddCreditsDialog })),
);

const Opener = createContext<(reais?: unknown) => void>(() => {});

/**
 * Opens the Add credits dialog, with `reais` typed in when it is a number (a
 * pricing card's amount); a no-op outside the shell. Anything else is ignored,
 * so it can be handed to an `onClick` as it is.
 */
export function useAddCredits(): (reais?: unknown) => void {
  return useContext(Opener);
}

export function AddCreditsProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [opened, setOpened] = useState(false);
  const [asked, setAsked] = useState<{ count: number; reais?: number }>({ count: 0 });
  return (
    <Opener.Provider
      value={(reais) => {
        // A new `count` is a new dialog, so the amount it starts with is the one asked for.
        setAsked(({ count }) => ({
          count: count + 1,
          reais: typeof reais === "number" ? reais : undefined,
        }));
        setOpened(true);
        setOpen(true);
      }}
    >
      {children}
      {opened && (
        <Suspense fallback={null}>
          <AddCreditsDialog
            key={asked.count}
            open={open}
            onOpenChange={setOpen}
            initial={asked.reais === undefined ? undefined : String(asked.reais)}
          />
        </Suspense>
      )}
    </Opener.Provider>
  );
}
