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

/// The time a stamp names, back from [`format`]'s text: `None` unless `word` is exactly one
/// stamp of a real date.
#[must_use]
pub fn parse(word: &str) -> Option<SystemTime> {
    if word.len() != WIDTH || !leads(word) {
        return None;
    }
    let num = |from: usize, to: usize| word[from..to].parse::<i64>().ok();
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second, milli) = (num(11, 13)?, num(14, 16)?, num(17, 19)?, num(20, 23)?);
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if day < 1 || civil_from_days(days) != (year, month, day) {
        return None;
    }
    let secs = u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second).ok()?;
    let millis = u64::try_from(milli).ok()?;
    UNIX_EPOCH.checked_add(Duration::from_secs(secs) + Duration::from_millis(millis))
}

/// Howard Hinnant's `days_from_civil`: (year, month, day) to days since 1970-01-01.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_reads_back_as_the_time_it_was_written_for() {
        for secs in [0_u64, 951_782_400, 1_763_456_096, 4_102_444_799] {
            let at = UNIX_EPOCH + Duration::from_millis(secs * 1000 + 789);
            assert_eq!(parse(&format(at)), Some(at), "{}", format(at));
        }
        assert_eq!(
            parse("2026-11-18T12:34:56.789Z").map(|t| format(t)),
            Some("2026-11-18T12:34:56.789Z".to_string())
        );
        for bad in [
            "2026-02-30T00:00:00.000Z",
            "2026-13-01T00:00:00.000Z",
            "2026-11-18T24:00:00.000Z",
            "2026-11-18T12:34:56.789Z trailing",
            "2026-11-18",
            "not a stamp at all here!",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }
}
