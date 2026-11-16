//! Distances round a lap: a craft's progress and a place ahead of it are both units from the
//! start line, and the line is between any two of them as often as not.

/// `progress` on by `units` (back, for a negative amount), taken round a lap of `length`:
/// in `0..length`. `progress` itself with no lap to take it round.
#[must_use]
pub fn ahead(progress: f32, units: f32, length: f32) -> f32 {
    if length.is_nan() || length <= 0.0 {
        return progress;
    }
    let at = progress + units;
    let laps = (at / length) as i32 as f32;
    let at = at - laps * length;
    if at < 0.0 { at + length } else { at }
}

/// How far on `to` is from `from` the short way round a lap of `length`, in
/// `-length / 2..length / 2`: negative once `from` is past it. The plain difference with no
/// lap to take it round.
#[must_use]
pub fn gap(from: f32, to: f32, length: f32) -> f32 {
    let delta = to - from;
    if length.is_nan() || length <= 0.0 {
        return delta;
    }
    delta - (delta / length + if delta < 0.0 { -0.5 } else { 0.5 }) as i32 as f32 * length
}
