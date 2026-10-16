// What either picker (`PlatformPicker`, `StylePicker`) is handed: the menu,
// what is chosen, and how to choose.

import type { StyleMenu } from "@/api";

export interface Choosing {
  menu: StyleMenu;
  value: string | null;
  onChange: (id: string | null) => void;
}
