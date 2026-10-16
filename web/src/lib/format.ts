// Sizes, durations and dates as a person reads them in a file list. Numbers
// and dates take the page's language as their `Intl` locale (#704) — a
// Brazilian reads `1,5 MB` and `4 de out. de 2026` — and default to English.

import type { Language } from "@/i18n/language";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** `1_536_000` → `1.5 MB`. Decimal units, as file managers show them. */
export function formatBytes(bytes: number, locale: Language = "en"): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  const number = new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
  return `${number.format(value)} ${UNITS[unit]}`;
}

/**
 * A number in the page's language. `digits` fixes the decimals shown
 * (`1.5` → `1,5` in Portuguese); without it, `Intl`'s own default applies.
 */
export function formatNumber(
  value: number,
  { digits }: { digits?: number } = {},
  locale: Language = "en",
): string {
  const options =
    digits === undefined
      ? undefined
      : { minimumFractionDigits: digits, maximumFractionDigits: digits };
  return new Intl.NumberFormat(locale, options).format(value);
}

/**
 * How finely `formatClock` reads a time — the caller's call, because it
 * depends on what the time is. `floor` for a position (a playhead, a ruler
 * tick: at 59.9 s you are not at a minute yet), `round` for a length (a 59.6 s
 * clip runs `1:00`), `tenths` where a reading is finer than a second.
 */
export type Precision = "floor" | "round" | "tenths";

/**
 * Seconds as a clock reads them: `0:59`, `1:01:40`, or `0:04.5` in tenths.
 * Hours only when there are any, and the first field is never zero-padded.
 * The total is rounded before it is split, so a length never reads `0:60`.
 * The same rule as `scorsese_core::clock`, which the desktop app uses.
 */
export function formatClock(seconds: number, precision: Precision): string {
  const clamped = Number.isNaN(seconds) ? 0 : Math.max(0, seconds);
  const tenths = precision === "tenths" ? Math.round(clamped * 10) : null;
  const whole =
    tenths !== null
      ? Math.floor(tenths / 10)
      : precision === "floor"
        ? Math.floor(clamped)
        : Math.round(clamped);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor(whole / 60) % 60;
  const secs = String(whole % 60).padStart(2, "0");
  const text =
    hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${secs}` : `${minutes}:${secs}`;
  return tenths !== null ? `${text}.${tenths % 10}` : text;
}

/** Seconds since the epoch, in the page's language and the reader's time zone. */
export function formatDate(epochSeconds: number, locale: Language = "en"): string {
  const dateTime = new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" });
  return dateTime.format(new Date(epochSeconds * 1000));
}
