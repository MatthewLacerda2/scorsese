//! How big a file is, said the one way every surface says it (#1287).
//!
//! **Decimal units**: a megabyte is 10⁶ bytes, as file managers, the web app
//! (`web/src/lib/format.ts`, whose tests pin the same cases) and the hosted
//! server's render quota have it. One decimal under 100 of a unit, none from
//! 100 up: `1.5 MB`, `250 MB`.

/// The units a size is said in, each a thousand of the one before.
const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// `1_536_000` → `1.5 MB`; `999` → `999 B`; `250_000_000` → `250 MB`.
pub fn format_bytes(bytes: u64) -> String {
    // A float only to print: the count itself stays whole everywhere else.
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let digits = if unit == 0 || value >= 100.0 { 0 } else { 1 };
    format!("{value:.digits$} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    /// The cases `web/src/lib/format.test.ts` pins too.
    #[test]
    fn sizes_read_as_a_file_manager_shows_them() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1_000), "1.0 KB");
        assert_eq!(format_bytes(1_536_000), "1.5 MB");
        assert_eq!(format_bytes(99_400_000), "99.4 MB");
        assert_eq!(format_bytes(250_000_000), "250 MB");
        assert_eq!(format_bytes(20_000_000_000), "20.0 GB");
        assert_eq!(format_bytes(3_200_000_000_000), "3.2 TB");
        assert_eq!(format_bytes(1_200_000_000_000_000), "1200 TB");
    }

    /// A mebibyte is a little over a megabyte, never a megabyte.
    #[test]
    fn a_kilobyte_is_a_thousand_bytes() {
        assert_eq!(format_bytes(1_024), "1.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(21_474_836_480), "21.5 GB");
    }
}
