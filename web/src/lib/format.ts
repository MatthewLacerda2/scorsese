// Sizes, durations and dates as a person reads them in a file list.

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** `1_536_000` → `1.5 MB`. Decimal units, as file managers show them. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

/** `75.4` → `1:15`; an hour or more → `1:02:03`. */
export function formatDuration(seconds: number): string {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${secs}` : `${minutes}:${secs}`;
}

const dateTime = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

/** Seconds since the epoch, in the reader's own locale and time zone. */
export function formatDate(epochSeconds: number): string {
  return dateTime.format(new Date(epochSeconds * 1000));
}
