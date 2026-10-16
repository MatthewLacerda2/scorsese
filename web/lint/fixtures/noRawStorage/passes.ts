// Names that only look like storage: a type, a key, a string, a parameter.
export type Store = Pick<Storage, "getItem">;
export const settings = { localStorage: "a property of our own" };
export const word = "localStorage";
export function stored(storage: Store | undefined): string | null {
  return storage?.getItem("key") ?? null;
}
