export { cn } from "cn";

/** `value` held between `lower` and `upper`; `lower` wins if they cross. */
export function clamp(value: number, lower: number, upper: number): number {
  return Math.max(lower, Math.min(value, upper));
}
