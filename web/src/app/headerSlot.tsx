// A page's own room in the app header (#764). The shell draws an empty place
// between its nav and the balance, and a page that has something to put there
// — today only the editor: the project's name, Save as template, the frame
// shape and Render — portals it in with `<HeaderSlot>`. Every other page
// leaves it empty, so the shell never knows what any page puts in it.
//
// A portal rather than lifting state into the shell: what the editor puts
// there is wired to the editor's own state (the selection, the shape), and a
// portal keeps it a child of the editor in React's tree while it is drawn in
// the header. Server rendering has no target, so there it draws nothing.

import { createContext, type ReactNode, useContext } from "react";
import { createPortal } from "react-dom";

const Target = createContext<HTMLElement | null>(null);

/** Provides the header's element to the pages under it; the shell's job. */
export const HeaderSlotProvider = Target.Provider;

/** Draws `children` in the header while this is mounted. */
export function HeaderSlot({ children }: { children: ReactNode }) {
  const target = useContext(Target);
  return target ? createPortal(children, target) : null;
}
