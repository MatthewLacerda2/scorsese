// "Add credits" (#548) can be asked for from the header's balance popup and
// from the lines that say a balance is empty (#1244). One dialog serves them
// all: the shell mounts it, and any component under it calls `useAddCredits()`
// to open it. The dialog is a chunk of its own, fetched on the first open.

import { createContext, lazy, type ReactNode, Suspense, useContext, useState } from "react";

const AddCreditsDialog = lazy(() =>
  import("@/payments/AddCredits").then((module) => ({ default: module.AddCreditsDialog })),
);

const Opener = createContext<() => void>(() => {});

/** Opens the Add credits dialog; a no-op outside the shell. */
export function useAddCredits(): () => void {
  return useContext(Opener);
}

export function AddCreditsProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [opened, setOpened] = useState(false);
  return (
    <Opener.Provider
      value={() => {
        setOpened(true);
        setOpen(true);
      }}
    >
      {children}
      {opened && (
        <Suspense fallback={null}>
          <AddCreditsDialog open={open} onOpenChange={setOpen} />
        </Suspense>
      )}
    </Opener.Provider>
  );
}
