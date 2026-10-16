// One exported component, with what may sit beside it.
import { useState } from "react";

export const MAX_ITEMS = 3;
export type Props = { items: string[] };
type Hidden = { a: number };
export type { Hidden };

export function useCount() {
  return useState(0);
}

function Item({ name }: { name: string }) {
  return <li>{name}</li>;
}

export function List({ items }: Props) {
  return <ul>{items.slice(0, MAX_ITEMS).map((name) => <Item key={name} name={name} />)}</ul>;
}
