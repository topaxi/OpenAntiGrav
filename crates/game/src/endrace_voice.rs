//! The voice lines a title's end-of-race screens play: the finishing place on
//! `EndRace Results` and the medal jingle (Pulse: `EndRace Rewards`, HD: `EndRace
//! Photo`).
//!
//! Both triggers are read off each executable
//! (`docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`, "The end-of-race
//! voices"; `docs/ghidra/functions/ps3-hdfury-eu/endrace-voices.md`), and where
//! the two differ the difference is [`oag_title::ResultsLaw`], title data. This module is the decision and nothing else: given the race's mode
//! and outcome and which screen is on top this tick, which cue starts. The
//! session plays it through [`oag_sound::sfx::MenuSfx`].

use oag_sound::sfx::Cue;
use oag_tables::race_campaign::Medal;
use oag_title::{MedalScreen, ResultsLaw, TournamentPlace};

/// The screen of the end-of-race flow that is on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// `Race End Photo`, or `EndRace Menu`: neither plays a voice.
    Other,
    /// `EndRace Results`.
    Results,
    /// `EndRace Rewards`.
    Rewards,
}

/// When `EndRaceResults_Update` speaks the place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaceTrigger {
    /// On the first update tick of the screen, off the result's word 0:
    /// `g_game_mode` 3 (Single Race) and 8 (Eliminator).
    OnEntry(u8),
    /// At the first three-second page flip, off the result's word 1, the
    /// standings rank: Tournament. `OnEnter` leaves the page flag set, so the
    /// first flip lands on `ER_TOUR_STAN` and speaks the standing, not the leg.
    OnStandings(u8),
}

/// What one race's end-of-race screens say, and what they have said so far.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    law: ResultsLaw,
    place: Option<PlaceTrigger>,
    medal: Option<Medal>,
    screen: Screen,
    spoke_place: bool,
    standings_were_showing: bool,
}

/// The place line for a 1-based `place`: first to third, nothing past them
/// (`EndRaceResults_Update` tests `< 2`, `< 3`, `< 4` and `0 <` for the first).
#[must_use]
pub fn place_cue(place: u8) -> Option<Cue> {
    match place {
        1 => Some(Cue::ResultsFirst),
        2 => Some(Cue::ResultsSecond),
        3 => Some(Cue::ResultsThird),
        _ => None,
    }
}

/// The medal jingle (`EndRaceRewards_OnEnter`'s three plays).
#[must_use]
pub fn medal_cue(medal: Medal) -> Cue {
    match medal {
        Medal::Gold => Cue::MedalGold,
        Medal::Silver => Cue::MedalSilver,
        Medal::Bronze => Cue::MedalBronze,
    }
}

impl Voice {
    /// The voice of a race in `mode` under `law`.
    ///
    /// `place` is the player's 1-based finishing place (Single Race) or place
    /// in the kill table (Eliminator); `standings_place` their Tournament
    /// standings rank; `campaign` whether a campaign cell is in play and `medal`
    /// its medal (`None` outside a cell: the jingle plays only when one is in
    /// play). Every other mode - Head2Head included, which shares Single Race's
    /// constructor but not its mode number - speaks no place.
    #[must_use]
    pub fn new(
        law: ResultsLaw,
        mode: oag_race::Mode,
        place: Option<u8>,
        standings_place: Option<u8>,
        campaign: bool,
        medal: Option<Medal>,
    ) -> Self {
        let place = match mode {
            _ if campaign && !law.place_in_campaign => None,
            oag_race::Mode::SingleRace | oag_race::Mode::Eliminator => {
                place.map(PlaceTrigger::OnEntry)
            }
            oag_race::Mode::Tournament => match law.tournament {
                TournamentPlace::StandingsFlip => standings_place.map(PlaceTrigger::OnStandings),
                TournamentPlace::OnEntry => place.map(PlaceTrigger::OnEntry),
            },
            _ => None,
        };
        Self {
            law,
            place,
            medal: medal.filter(|_| campaign),
            screen: Screen::Other,
            spoke_place: false,
            standings_were_showing: false,
        }
    }

    /// One tick: the screen on top, and whether a Tournament's results are on
    /// their standings page. Returns the cue that starts this tick, if any.
    ///
    /// `VIEW RESULTS AGAIN` speaks the place again: **chosen, not measured**. The
    /// original's once-flag (`+0xe8`) is never written by `OnEnter`, and whether the
    /// widget is rebuilt on a second entry was not read.
    pub fn tick(&mut self, screen: Screen, standings_showing: bool) -> Option<Cue> {
        let entered = screen != self.screen;
        self.screen = screen;
        match screen {
            Screen::Other => None,
            Screen::Results => {
                if entered
                    && self.law.medal_screen == MedalScreen::Results
                    && let Some(medal) = self.medal
                {
                    return Some(medal_cue(medal));
                }
                if entered {
                    self.spoke_place = false;
                    self.standings_were_showing = standings_showing;
                }
                let rising = standings_showing && !self.standings_were_showing;
                self.standings_were_showing = standings_showing;
                let due = match self.place? {
                    PlaceTrigger::OnEntry(place) => (!self.spoke_place).then_some(place),
                    PlaceTrigger::OnStandings(place) => {
                        (rising && !self.spoke_place).then_some(place)
                    }
                };
                let cue = place_cue(due?);
                // The original sets its flag only when a line played.
                self.spoke_place |= cue.is_some();
                cue
            }
            Screen::Rewards => (entered && self.law.medal_screen == MedalScreen::Rewards)
                .then(|| self.medal.map(medal_cue))
                .flatten(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_race::Mode;

    const PULSE: ResultsLaw = ResultsLaw {
        tournament: TournamentPlace::StandingsFlip,
        place_in_campaign: true,
        medal_screen: MedalScreen::Rewards,
    };
    const HD: ResultsLaw = ResultsLaw {
        tournament: TournamentPlace::OnEntry,
        place_in_campaign: false,
        medal_screen: MedalScreen::Results,
    };

    fn speak(voice: &mut Voice, screens: &[(Screen, bool)]) -> Vec<Cue> {
        screens
            .iter()
            .filter_map(|&(screen, standings)| voice.tick(screen, standings))
            .collect()
    }

    #[test]
    fn a_single_race_speaks_its_place_once_on_the_first_results_tick() {
        let mut voice = Voice::new(PULSE, Mode::SingleRace, Some(2), None, false, None);
        let mut screens = vec![(Screen::Other, false)];
        screens.extend([(Screen::Results, false); 6]);
        let said = speak(&mut voice, &screens);
        assert_eq!(said, [Cue::ResultsSecond]);
    }

    #[test]
    fn fourth_and_worse_say_nothing_and_do_not_arm_the_flag() {
        assert_eq!(place_cue(0), None);
        assert_eq!(place_cue(4), None);
        let mut voice = Voice::new(PULSE, Mode::Eliminator, Some(8), None, false, None);
        assert_eq!(speak(&mut voice, &[(Screen::Results, false); 3]), []);
    }

    #[test]
    fn only_single_race_eliminator_and_tournament_speak_a_place() {
        for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone, Mode::Head2Head] {
            let mut voice = Voice::new(PULSE, mode, Some(1), Some(1), false, None);
            assert_eq!(
                speak(&mut voice, &[(Screen::Results, true); 2]),
                [],
                "{mode:?}"
            );
        }
        let mut voice = Voice::new(PULSE, Mode::Eliminator, Some(1), None, false, None);
        assert_eq!(
            speak(&mut voice, &[(Screen::Results, false)]),
            [Cue::ResultsFirst]
        );
    }

    #[test]
    fn a_tournament_speaks_its_standing_at_the_first_flip_not_on_entry() {
        let mut voice = Voice::new(PULSE, Mode::Tournament, Some(1), Some(3), false, None);
        let mut said = speak(&mut voice, &[(Screen::Results, false); 4]);
        assert_eq!(said, [], "the leg page speaks nothing");
        said = speak(
            &mut voice,
            &[(Screen::Results, true), (Screen::Results, false)],
        );
        assert_eq!(said, [Cue::ResultsThird]);
        said = speak(&mut voice, &[(Screen::Results, true); 2]);
        assert_eq!(said, [], "once per entry");
    }

    #[test]
    fn leaving_and_re_entering_results_speaks_again() {
        let mut voice = Voice::new(PULSE, Mode::SingleRace, Some(1), None, false, None);
        let said = speak(
            &mut voice,
            &[
                (Screen::Results, false),
                (Screen::Other, false),
                (Screen::Results, false),
            ],
        );
        assert_eq!(said, [Cue::ResultsFirst, Cue::ResultsFirst]);
    }

    #[test]
    fn rewards_plays_the_medal_on_entry_and_only_when_one_was_won() {
        let mut voice = Voice::new(
            PULSE,
            Mode::SingleRace,
            Some(5),
            None,
            true,
            Some(Medal::Silver),
        );
        let said = speak(&mut voice, &[(Screen::Rewards, false); 3]);
        assert_eq!(said, [Cue::MedalSilver]);
        let mut none = Voice::new(PULSE, Mode::SingleRace, Some(5), None, false, None);
        assert_eq!(speak(&mut none, &[(Screen::Rewards, false); 3]), []);
        assert_eq!(medal_cue(Medal::Gold), Cue::MedalGold);
        assert_eq!(medal_cue(Medal::Bronze), Cue::MedalBronze);
    }

    #[test]
    fn hd_speaks_a_tournament_legs_place_on_entry_not_the_standings() {
        let mut voice = Voice::new(HD, Mode::Tournament, Some(2), Some(3), false, None);
        let said = speak(
            &mut voice,
            &[(Screen::Results, false), (Screen::Results, true)],
        );
        assert_eq!(said, [Cue::ResultsSecond]);
    }

    #[test]
    fn hd_speaks_the_medal_not_the_place_in_a_campaign_cell() {
        let mut voice = Voice::new(HD, Mode::SingleRace, Some(1), None, true, Some(Medal::Gold));
        let said = speak(&mut voice, &[(Screen::Results, false); 4]);
        assert_eq!(said, [Cue::MedalGold], "the place line is silent in a cell");
        let mut lost = Voice::new(HD, Mode::SingleRace, Some(1), None, true, None);
        assert_eq!(speak(&mut lost, &[(Screen::Results, false); 4]), []);
    }

    #[test]
    fn hd_has_no_rewards_voice_and_pulse_has_no_medal_on_results() {
        let mut hd = Voice::new(HD, Mode::SingleRace, Some(5), None, true, Some(Medal::Gold));
        assert_eq!(speak(&mut hd, &[(Screen::Rewards, false)]), []);
        let mut pulse = Voice::new(
            PULSE,
            Mode::SingleRace,
            Some(5),
            None,
            true,
            Some(Medal::Gold),
        );
        assert_eq!(speak(&mut pulse, &[(Screen::Results, false)]), []);
    }

    #[test]
    fn hd_speaks_a_free_single_race_place_once() {
        let mut voice = Voice::new(HD, Mode::SingleRace, Some(3), None, false, None);
        let said = speak(&mut voice, &[(Screen::Results, false); 5]);
        assert_eq!(said, [Cue::ResultsThird]);
    }
}
