//! The arithmetic behind a whole-race bench's result file: percentiles,
//! per-metric summaries, and the JSON text they are written as. No GPU, no
//! race, so it is unit-tested without either.

/// One metric over a run: how many samples and where they sit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Summary {
    pub count: usize,
    pub min: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

/// The `p`th percentile (0..=100) of an ascending slice, by nearest rank: the
/// smallest sample at least `p` percent of the samples are at or below. A rank
/// rather than an interpolation, so every reported value is a sample that
/// really happened - a p99 of 2,000 frames is the 1,980th-slowest frame.
///
/// `None` for an empty slice.
pub(super) fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a rank within a slice length"
    )]
    let rank = ((p / 100.0 * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

/// Summarises the finite samples of `values`; a missing reading (`NaN`) is
/// skipped, not counted as zero. `None` when nothing is left.
pub(super) fn summarise(values: &[f64]) -> Option<Summary> {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    #[expect(clippy::cast_precision_loss, reason = "a sample count")]
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    Some(Summary {
        count: sorted.len(),
        min: sorted[0],
        mean,
        p50: percentile(&sorted, 50.0)?,
        p95: percentile(&sorted, 95.0)?,
        p99: percentile(&sorted, 99.0)?,
        max: sorted[sorted.len() - 1],
    })
}

impl Summary {
    /// `{"count": .., "min": .., ...}`, numbers to a tenth of a unit.
    pub(super) fn json(&self) -> String {
        format!(
            "{{\"count\": {}, \"min\": {:.1}, \"mean\": {:.1}, \"p50\": {:.1}, \"p95\": {:.1}, \"p99\": {:.1}, \"max\": {:.1}}}",
            self.count, self.min, self.mean, self.p50, self.p95, self.p99, self.max
        )
    }
}

/// `text` as a JSON string literal, quotes included.
pub(super) fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_returns_a_real_sample() {
        let values: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&values, 50.0), Some(50.0));
        assert_eq!(percentile(&values, 95.0), Some(95.0));
        assert_eq!(percentile(&values, 99.0), Some(99.0));
        assert_eq!(percentile(&values, 100.0), Some(100.0));
        assert_eq!(percentile(&values, 0.0), Some(1.0));
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn a_small_sample_rounds_the_rank_up() {
        // Four samples: p50 is the 2nd, p95 and p99 the 4th (the slowest).
        let values = [10.0, 20.0, 30.0, 40.0];
        assert_eq!(percentile(&values, 50.0), Some(20.0));
        assert_eq!(percentile(&values, 95.0), Some(40.0));
    }

    #[test]
    fn missing_readings_are_skipped_not_zeroed() {
        let summary = summarise(&[f64::NAN, 4.0, 2.0, f64::NAN, 6.0]).unwrap();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.min, 2.0);
        assert_eq!(summary.max, 6.0);
        assert_eq!(summary.mean, 4.0);
        assert_eq!(summary.p50, 4.0);
        assert!(summarise(&[f64::NAN]).is_none());
    }

    #[test]
    fn the_summary_is_valid_json_shaped_text() {
        let text = summarise(&[1.0, 2.0, 3.0]).unwrap().json();
        assert!(text.starts_with("{\"count\": 3,"));
        assert!(text.contains("\"p99\": 3.0"));
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
        assert_eq!(json_string("\u{1}"), "\"\\u0001\"");
    }
}
