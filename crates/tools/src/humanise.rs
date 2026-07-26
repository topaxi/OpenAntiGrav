//! Formatting helpers for human-readable tool output.

/// Formats a byte count with a binary unit suffix.
///
/// Listings are read by people scanning for the big files, and
/// `1.4 GiB` answers that at a glance where `1503238553` does not.
#[must_use]
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    if n < 1024 {
        return format!("{n} B");
    }

    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    // One decimal below 10 units, none above: `9.8 MiB` but `512 MiB`.
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_small_values_exactly() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1), "1 B");
        assert_eq!(bytes(1023), "1023 B");
    }

    #[test]
    fn switches_unit_at_the_boundary() {
        assert_eq!(bytes(1024), "1.0 KiB");
        assert_eq!(bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(bytes(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn drops_the_decimal_above_ten_units() {
        assert_eq!(bytes(512 * 1024), "512 KiB");
        assert_eq!(bytes(1536), "1.5 KiB");
    }

    #[test]
    fn saturates_at_the_largest_unit() {
        assert!(bytes(u64::MAX).ends_with("TiB"));
    }
}
