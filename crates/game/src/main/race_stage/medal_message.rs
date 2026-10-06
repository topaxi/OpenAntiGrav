//! The message a Zone or Speed Lap race raises the moment it earns a medal.
//!
//! **Chosen, not measured.** The original raises no such message: its race HUD
//! evaluates a medal nowhere (`Cell_EvaluateMedal`'s callers are
//! `Race_RecordResult` and two menus), and its message lines
//! (`Hud_UpdateMessages`, `oag_hud::messages`) name eight events, none of them a
//! medal. The maintainer asked for one - a Zone run or a Speed Lap goes on after
//! gold, and nothing on screen said a medal had been earned - so this build
//! raises it, through the original's own presentation (the four `Info` lines,
//! their fade and colour) and with the original's own end-of-race phrase
//! (`ER_GMA`/`ER_SMA`/`ER_BMA`, "Gold medal awarded") rather than invented text.
//!
//! What counts is [`RaceStage::campaign_medal`] unchanged, asked mid-race: the
//! same `Cell_EvaluateMedal` law the end-of-race record uses, so the banner
//! can never promise a medal the results then withhold. Time Trial earns its
//! medal at the finish and stays silent here; its live pace caption already
//! tells the player where they stand.

use oag_game::records::Medal;
use oag_tables::race_campaign::Mode;

use super::RaceStage;

impl RaceStage {
    /// Raises the HUD message for a medal earned this tick. Called straight
    /// after `Race::tick`, so a lap or a zone that lands on this tick is
    /// announced on it; the line itself is the race's
    /// ([`oag_raceplay::Race::raise_message`]), advanced by `Race::tick`.
    pub(crate) fn tick_messages(&mut self) {
        if let Some(medal) = self.newly_earned_medal() {
            self.race.raise_message(phrase(medal), true);
        }
    }

    /// The medal this race has just earned over the one it last announced, if
    /// any. A medal only ever improves: a worse one, or the same one, is not
    /// news.
    fn newly_earned_medal(&mut self) -> Option<Medal> {
        let cell = self.campaign_cell.as_ref()?;
        if !matches!(cell.mode, Mode::Zone | Mode::SpeedLap) {
            return None;
        }
        let best_lap = self.race.player_standing().best_lap_ticks;
        let announce = improvement(self.earned_medal, self.campaign_medal(false, best_lap));
        if let Some(medal) = announce {
            self.earned_medal = Some(medal);
        }
        announce
    }
}

/// `now` where it beats `had` (gold beats silver beats bronze, anything beats
/// nothing), else `None`: a medal only ever improves, and the same one twice
/// is not news.
fn improvement(had: Option<Medal>, now: Option<Medal>) -> Option<Medal> {
    let now = now?;
    had.is_none_or(|had| now < had).then_some(now)
}

/// The language-table id of the phrase, which the HUD resolves when it draws.
fn phrase(medal: Medal) -> &'static str {
    match medal {
        Medal::Gold => "ER_GMA",
        Medal::Silver => "ER_SMA",
        Medal::Bronze => "ER_BMA",
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
}
