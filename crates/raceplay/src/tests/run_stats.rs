//! [`RunStats`], the Zone results screen's two tallies.

use crate::results::RunStats;

/// `Zone_Update` keeps `u16(speed * 100)` and the screen prints `top * 3600 /
/// 100000`: a craft at 100 units a second reads 360 km/h - the speedometer's own 3.6.
#[test]
fn top_speed_is_the_zone_updates_hundredths_and_the_screens_integer_kmh() {
    let mut stats = RunStats::default();
    assert_eq!(stats.top_speed_kmh(), 0);
    stats.observe(100.0);
    assert_eq!(stats.top_speed_kmh(), 360);
    // A slower moment does not lower it.
    stats.observe(40.0);
    assert_eq!(stats.top_speed_kmh(), 360);
    // 226.4 u/s: 22640 hundredths, 22640 * 3600 / 100000 = 815 (truncated).
    stats.observe(226.4);
    assert_eq!(stats.top_speed_kmh(), 815);
}

/// The stored value is a `u16`: `(uint)(speed * 100)` masked to sixteen bits, so a
/// figure past 655.35 wraps as the original's does rather than saturating.
#[test]
fn the_stored_speed_is_sixteen_bits_wide() {
    let mut stats = RunStats::default();
    stats.observe(700.0);
    // 70000 & 0xffff = 4464 hundredths.
    assert_eq!(stats.top_speed_kmh(), 4464 * 3600 / 100_000);
}
