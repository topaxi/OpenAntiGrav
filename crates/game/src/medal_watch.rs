//! The medal a Zone or Speed Lap race has earned so far, and what the HUD
//! says about it.
//!
//! **Chosen, not measured.** The original evaluates a medal nowhere in its
//! race HUD (`Cell_EvaluateMedal`'s callers are `Race_RecordResult` and two
//! menus), so what a race shows mid-race here is this build's own, asked for
//! by the maintainer: a Zone run or a Speed Lap goes on after gold and the
//! player could not tell which medal was already safe. Two things come of it:
//!
//! - the tick a tier is first reached (and again on each better tier), a
//!   four-second line through the HUD's own message slots
//!   (`oag_hud::messages`) saying the disc's own end-of-race phrase
//!   (`ER_GMA`/`ER_SMA`/`ER_BMA`);
//! - a **standing** line in the last of those slots, which stays for the rest
//!   of the race in the layout's own text colour: the words name the tier, so
//!   no colour of this build's own is added to them.
//!
//! The test is the same `Cell_EvaluateMedal` law the results use
//! ([`oag_tables::race_campaign::Cell::evaluate_medal_for_difficulty`]) over
//! the race's own value, so the HUD can never promise a medal the results then
//! withhold. It lives here, below the window and the GPU, so the windowed
//! session and a headless capture call the same function - a test that drives
//! a race through it fails when the call is dropped from either.

use oag_raceplay::Race;
use oag_tables::race_campaign::{Cell, Difficulty, Medal, Mode};

/// 60 Hz ticks to centiseconds, the unit a campaign cell's own `Gold`/
/// `Silver`/`Bronze Target` is authored in for `Time Trial`/`Speed Lap`.
/// **Chosen, not measured**: nothing traces the original's own
/// tick-to-centisecond rounding rule, only that ticks run at the fixed 60 Hz
/// ADR-0007 mandates.
#[must_use]
pub fn ticks_to_centiseconds(ticks: u64) -> i64 {
    i64::try_from(ticks.saturating_mul(5) / 3).unwrap_or(i64::MAX)
}

/// The value `Cell_EvaluateMedal` is asked about for a Zone or Speed Lap race
/// right now: the zone reached, or the best lap so far in centiseconds. `None`
/// for any other mode, and for a Speed Lap with no lap yet.
#[must_use]
pub fn live_value(mode: &Mode, race: &Race) -> Option<i64> {
    match mode {
        Mode::Zone => Some(i64::from(race.sim.world.primary_race().zone)),
        Mode::SpeedLap => race
            .player_standing()
            .best_lap_ticks
            .map(|ticks| ticks_to_centiseconds(u64::from(ticks))),
        _ => None,
    }
}

/// The language-table id of a tier's end-of-race phrase.
#[must_use]
pub fn phrase(medal: Medal) -> &'static str {
    match medal {
        Medal::Gold => "ER_GMA",
        Medal::Silver => "ER_SMA",
        Medal::Bronze => "ER_BMA",
    }
}

/// `now` where it beats `had` (gold beats silver beats bronze, anything beats
/// nothing), else `None`: a medal only ever improves, and the same one twice is
/// not news.
#[must_use]
pub fn improvement(had: Option<Medal>, now: Option<Medal>) -> Option<Medal> {
    let now = now?;
    had.is_none_or(|had| now < had).then_some(now)
}

/// One tick of the watch, after `Race::tick`: if the race has just earned a
/// better medal than `earned`, records it, raises the four-second line and
/// updates the standing one. Returns the new tier.
pub fn tick(
    cell: &Cell,
    difficulty: Difficulty,
    earned: &mut Option<Medal>,
    race: &mut Race,
) -> Option<Medal> {
    let value = live_value(&cell.mode, race)?;
    let now = cell.evaluate_medal_for_difficulty(value, difficulty);
    let better = improvement(*earned, now)?;
    *earned = Some(better);
    race.raise_message(phrase(better), true);
    race.set_standing_message(phrase(better));
    Some(better)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bronze on the first Speed Lap, silver on a better one, gold on the best:
    /// three announcements. A slower lap, or the same tier again, raises nothing.
    #[test]
    fn only_a_better_medal_is_announced() {
        let (g, s, b) = (Medal::Gold, Medal::Silver, Medal::Bronze);
        assert_eq!(improvement(None, None), None);
        assert_eq!(improvement(None, Some(b)), Some(b));
        assert_eq!(improvement(Some(b), Some(b)), None);
        assert_eq!(improvement(Some(b), Some(s)), Some(s));
        assert_eq!(improvement(Some(s), Some(b)), None);
        assert_eq!(improvement(Some(s), Some(g)), Some(g));
        assert_eq!(improvement(Some(g), Some(g)), None);
        assert_eq!(improvement(Some(g), None), None);
    }

    #[test]
    fn each_tier_names_its_own_end_of_race_phrase() {
        assert_eq!(phrase(Medal::Gold), "ER_GMA");
        assert_eq!(phrase(Medal::Silver), "ER_SMA");
        assert_eq!(phrase(Medal::Bronze), "ER_BMA");
    }

    #[test]
    fn a_centisecond_is_one_sixtieth_scaled() {
        assert_eq!(ticks_to_centiseconds(60), 100);
        assert_eq!(ticks_to_centiseconds(0), 0);
    }
}
