//! The timestamp every log line starts with: `2026-10-06T12:34:56.789Z`.
//!
//! Fixed width and UTC, so the text sorts as time does and the prune can compare
//! a line's first [`WIDTH`] bytes against a cutoff written the same way. Hand
//! rolled (days-from-civil) because the only other need for a calendar in this
//! workspace is none: a date crate for one format string is a dependency
//! `cargo shear` would be right to question.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Bytes in a stamp.
pub const WIDTH: usize = 24;

/// The stamp for `at`. A time before 1970 reads as the epoch.
#[must_use]
pub fn format(at: SystemTime) -> String {
    let since = at.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
    let secs = since.as_secs();
    let (year, month, day) = civil_from_days(i64::try_from(secs / 86_400).unwrap_or(0));
    let rem = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        since.subsec_millis()
    )
}

/// Whether `line` starts with a stamp, which is what makes it an entry rather
/// than the continuation of the one before.
#[must_use]
pub fn leads(line: &str) -> bool {
    let b = line.as_bytes();
    if b.len() < WIDTH {
        return false;
    }
    b[..WIDTH].iter().enumerate().all(|(i, &c)| match i {
        4 | 7 => c == b'-',
        10 => c == b'T',
        13 | 16 => c == b':',
        19 => c == b'.',
        23 => c == b'Z',
        _ => c.is_ascii_digit(),
    })
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to (year, month, day).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}
