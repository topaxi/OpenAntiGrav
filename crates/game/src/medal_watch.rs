//! The medal a Zone or Speed Lap race has earned so far, and what the HUD
//! says about it.
//!
//! **Chosen, not measured.** Pulse evaluates a medal nowhere in its race HUD
//! (`Cell_EvaluateMedal`'s callers are `Race_RecordResult` and two menus), so
//! what a race shows mid-race here is this build's own, added because
//! a Zone run or a Speed Lap goes on after gold and the player could
//! not tell which medal was already safe. **HD does raise one, in Speed Lap**: on
//! every lap that is a new race-best and inside a tier, with a 3D trophy and a
//! voice (`docs/ghidra/functions/ps3-hdfury-eu/hud-message-triggers.md`). A title
//! that names that rule ([`oag_title::lap_hud::SpeedLapMedal`], HD) gets it here
//! as measured, voice included and trophy not drawn; every other title's watch
//! raises on a tier improvement only (chosen). Two things come of it:
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

/// The value an `Elimination` campaign cell's medal is evaluated on.
///
/// A title that flips Elimination (`counting` names it; Pulse) or a mod campaign's own kill
/// targets (`mod_targets`) read the kills so far, which only grow. Every other title (HD,
/// Omega) reads the player's place on the end-of-race board, and only once the target ended the
/// race (`finished`): the original records nothing for a quit Eliminator
/// (`docs/ghidra/functions/ps3-hdfury-eu/cell-medal-award.md`, "End of race").
#[must_use]
pub fn elimination_value(
    counting: &[&str],
    mod_targets: bool,
    finished: bool,
    kills: u32,
    place: u8,
) -> Option<i64> {
    if mod_targets || counting.contains(&Mode::Elimination.as_str()) {
        Some(i64::from(kills))
    } else {
        finished.then(|| i64::from(place))
    }
}

/// The language-table id of a tier's end-of-race phrase.
#[must_use]
pub fn phrase(medal: Medal) -> &'static str {
    match medal {
        Medal::Gold => "ER_GMA",
        Medal::Silver => "ER_SMA",
        _ => "ER_BMA",
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
///
/// `counting` is the title's [`oag_title::Campaign::counting_modes`]. Only a Zone or Speed Lap
/// race is watched: an Eliminator cell's medal is its place on the end-of-race board, which
/// does not exist mid-race (the original records only at the end).
pub fn tick(
    cell: &Cell,
    difficulty: Difficulty,
    counting: &[&str],
    earned: &mut Option<Medal>,
    race: &mut Race,
) -> Option<Medal> {
    if cell.mode == Mode::SpeedLap && race.speed_lap_medal_rule().is_some() {
        return every_best_lap(cell, difficulty, counting, earned, race);
    }
    let value = live_value(&cell.mode, race)?;
    let now = cell.evaluate_medal_for_difficulty(value, difficulty, counting);
    let better = improvement(*earned, now)?;
    *earned = Some(better);
    race.raise_message(phrase(better), true);
    race.set_standing_message(phrase(better));
    Some(better)
}

/// HD's Speed Lap block (`Craft_UpdateLapProgress` `0x000e553c`): every lap that
/// is a new race-best and inside a tier raises the tier's line and plays its
/// jingle, whether or not the tier improved. The standing line still follows
/// the best tier (chosen, not measured; the original shows a 3D trophy there).
fn every_best_lap(
    cell: &Cell,
    difficulty: Difficulty,
    counting: &[&str],
    earned: &mut Option<Medal>,
    race: &mut Race,
) -> Option<Medal> {
    let ticks = race.take_new_best_lap()?;
    let value = ticks_to_centiseconds(u64::from(ticks));
    let now = cell.evaluate_medal_for_difficulty(value, difficulty, counting)?;
    race.raise_message(phrase(now), true);
    race.raise_cue(voice(now));
    if improvement(*earned, Some(now)).is_some() {
        *earned = Some(now);
        race.set_standing_message(phrase(now));
    }
    Some(now)
}

/// The cue a tier's jingle is.
#[must_use]
pub fn voice(medal: Medal) -> oag_gameplay::cue::Cue {
    use oag_gameplay::cue::Cue;
    match medal {
        Medal::Gold => Cue::SpeedLapGold,
        Medal::Silver => Cue::SpeedLapSilver,
        _ => Cue::SpeedLapBronze,
    }
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

    const PULSE: &[&str] = &["Zone", "Elimination"];
    const HD: &[&str] = &["Zone", "Detonator"];

    /// HD reads the place, and only on a finished race; Pulse and a mod campaign read kills.
    #[test]
    fn an_elimination_cell_is_read_by_place_on_hd_and_by_kills_on_pulse() {
        assert_eq!(elimination_value(HD, false, true, 6, 2), Some(2));
        assert_eq!(elimination_value(HD, false, false, 6, 2), None);
        assert_eq!(elimination_value(PULSE, false, false, 6, 2), Some(6));
        assert_eq!(elimination_value(HD, true, true, 6, 2), Some(6));
    }

    /// End to end on the real cell law: place 1, 2, 3, 4 on an HD cell is gold, silver, bronze,
    /// nothing, where the same cell read by kills (6) would be nothing at all.
    #[test]
    fn the_hd_value_through_the_cell_law_is_gold_silver_bronze_none() {
        let grid = oag_tables::race_campaign::parse(
            r#"<PI_Grid name="grid8"><Values RequiredPoints="0" Locked="false"></Values>
<PI_Cell name="grid8_3_2"><Values track="88_Track" mode="Elimination" class="Flash" Weapons="on" damage="on" AICount="7" skillMedium="1.8" ship="None" ShipChoice="Yes"></Values>
<EasyGold Target="1"></EasyGold><EasySilver Target="2"></EasySilver><EasyBronze Target="3"></EasyBronze>
<MediumGold Target="1"></MediumGold><MediumSilver Target="2"></MediumSilver><MediumBronze Target="3"></MediumBronze>
<HardGold Target="1"></HardGold><HardSilver Target="2"></HardSilver><HardBronze Target="3"></HardBronze>
</PI_Cell></PI_Grid>"#,
        )
        .expect("parses");
        let cell = &grid.cells[0];
        let medals: Vec<_> = (1..=4u8)
            .map(|place| {
                elimination_value(HD, false, true, 6, place)
                    .and_then(|v| cell.evaluate_medal(v, HD))
            })
            .collect();
        assert_eq!(
            medals,
            [
                Some(Medal::Gold),
                Some(Medal::Silver),
                Some(Medal::Bronze),
                None
            ]
        );
        assert_eq!(
            elimination_value(HD, false, true, 6, 4).and_then(|v| cell.evaluate_medal(v, HD)),
            None
        );
    }
}
