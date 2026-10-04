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

/** `75.4` → `1:15`; an hour or more → `1:02:03`. */
export function formatDuration(seconds: number): string {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${secs}` : `${minutes}:${secs}`;
}

/** Seconds since the epoch, in the page's language and the reader's time zone. */
export function formatDate(epochSeconds: number, locale: Language = "en"): string {
  const dateTime = new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" });
  return dateTime.format(new Date(epochSeconds * 1000));
}
