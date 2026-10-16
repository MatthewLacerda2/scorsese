// Every line marked `flagged` must be reported, and no other.
import { createContext, memo } from "react";

const Slot = createContext<string>("");

export function Card() { // flagged
  return <div />;
}

export const Row = memo(() => <span />); // flagged

export const SlotProvider = Slot.Provider; // flagged

export default function Page() { // flagged
  return <Card />;
}

function Badge() {
  return <b />;
}

export { Badge, Badge as Chip }; // flagged
