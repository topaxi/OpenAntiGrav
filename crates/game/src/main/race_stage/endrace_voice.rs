//! Which voice lines a finished race's end-of-race screens speak: the place
//! and medal inputs read off the models the screens were built from, handed to
//! [`oag_game::endrace_voice::Voice`], which decides the tick.

use oag_game::endrace_voice::Voice;
use oag_tables::race_campaign::Medal;
use oag_title::ResultsLaw;

use super::ResultsModel;
use super::endrace_flow::Which;

/// The voice for a race in `mode` whose screens hold `results`, under the
/// title's `law` ([`None`] where the title names no results bank).
///
/// The place is the one `g_endrace_result` word 0 carries: the finishing place
/// (`race_place`), or on Pulse the player's place in the Eliminator's own ranked
/// table. Pulse's Tournament speaks word 1, the player's standings rank, which is
/// their index in the standings rows; HD's speaks the leg's place. The medal counts
/// only in a campaign cell (`campaign`).
#[must_use]
pub(crate) fn voice_for(
    law: Option<ResultsLaw>,
    mode: oag_race::Mode,
    results: &ResultsModel,
    race_place: Option<u8>,
    campaign: bool,
    medal: Option<Medal>,
) -> Option<Voice> {
    let (place, standings_place) = match results {
        ResultsModel::Hd(_) => (race_place, None),
        ResultsModel::PulseElimination(table) => (table.place(), None),
        ResultsModel::PulseTournament(table) => (
            race_place,
            table
                .standings
                .iter()
                .position(|row| row.player)
                .and_then(|index| u8::try_from(index + 1).ok()),
        ),
        ResultsModel::Pulse(_) | ResultsModel::PulseZone(_) => (race_place, None),
        // Several people: nobody's place is "the" place, so nothing is announced.
        ResultsModel::PulsePeople(_) | ResultsModel::PulseSplit(_) => return None,
    };
    Some(Voice::new(
        law?,
        mode,
        place,
        standings_place,
        campaign,
        medal,
    ))
}

/// The voice a finished race's screens speak, and the screen-on-top reading
/// that decides each tick whether it sounds. GPU-free, so the order of the
/// screens and the lines they speak are tested without a device; the runtime
/// forwards one call per tick.
#[derive(Debug)]
pub(crate) struct Speaker {
    voice: Option<Voice>,
}

impl Speaker {
    #[must_use]
    pub(crate) fn new(voice: Option<Voice>) -> Self {
        Self { voice }
    }

    /// The voice cue that starts this tick, if the screen on top calls for
    /// one. `results` says whether a Tournament's standings page is showing.
    pub(super) fn tick(
        &mut self,
        which: Which,
        results: &ResultsModel,
    ) -> Option<oag_sound::sfx::Cue> {
        use oag_game::endrace_voice::Screen;
        let screen = match which {
            Which::Results => Screen::Results,
            Which::Rewards => Screen::Rewards,
            Which::Photo | Which::Menu => Screen::Other,
        };
        let standings = matches!(results,
            ResultsModel::PulseTournament(table) if table.showing_standings());
        self.voice.as_mut()?.tick(screen, standings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_game::endrace_voice::Screen;
    use oag_sound::sfx::Cue;
    use oag_tables::race_campaign::Medal;
    use oag_ui_screens::endrace::{
        EliminationRow, Headline, Results, TournamentResults, TournamentRow,
    };

    fn pulse() -> Option<ResultsLaw> {
        oag_pulse::race::SOUND_BANKS
            .frontend
            .and_then(|front| front.results)
            .map(|results| results.law)
    }

    fn spoken(mut voice: Voice, screen: Screen, standings: bool) -> Option<Cue> {
        voice.tick(screen, standings)
    }

    fn row(player: bool) -> TournamentRow {
        TournamentRow {
            team_name: None,
            points: 0,
            player,
        }
    }

    #[test]
    fn a_single_race_speaks_the_finishing_place() {
        let results = ResultsModel::Pulse(Results {
            headline: Headline::Position(3),
            laps: Vec::new(),
            total_ticks: 0,
        });
        let voice = voice_for(pulse(), oag_race::Mode::SingleRace, &results, Some(3), false, None).unwrap();
        assert_eq!(
            spoken(voice, Screen::Results, false),
            Some(Cue::ResultsThird)
        );
    }

    #[test]
    fn the_eliminator_speaks_the_place_in_its_own_ranked_table_not_the_board() {
        let table = |kills_of_player| {
            ResultsModel::PulseElimination(oag_ui_screens::endrace::EliminationResults::new(vec![
                EliminationRow {
                    team_name: None,
                    kills: kills_of_player,
                    deaths: 0,
                    player: true,
                },
                EliminationRow {
                    team_name: None,
                    kills: 2,
                    deaths: 0,
                    player: false,
                },
            ]))
        };
        // The race place is 8 here; the table ranks the player first.
        let first = voice_for(pulse(), oag_race::Mode::Eliminator, &table(5), Some(8), false, None).unwrap();
        assert_eq!(
            spoken(first, Screen::Results, false),
            Some(Cue::ResultsFirst)
        );
        let second = voice_for(pulse(), oag_race::Mode::Eliminator, &table(1), Some(8), false, None).unwrap();
        assert_eq!(
            spoken(second, Screen::Results, false),
            Some(Cue::ResultsSecond)
        );
    }

    #[test]
    fn a_tournament_speaks_the_players_standings_index_at_the_flip() {
        let results = ResultsModel::PulseTournament(TournamentResults::new(
            false,
            1,
            3,
            vec![row(true), row(false), row(false)],
            vec![row(false), row(false), row(true)],
        ));
        let mut voice = voice_for(pulse(), oag_race::Mode::Tournament, &results, Some(1), false, None).unwrap();
        assert_eq!(voice.tick(Screen::Results, false), None, "not on entry");
        assert_eq!(
            voice.tick(Screen::Results, true),
            Some(Cue::ResultsThird),
            "the standings are third, the leg first"
        );
    }

    #[test]
    fn the_medal_needs_a_campaign_cell_and_a_title_without_a_law_speaks_nothing() {
        let results = ResultsModel::Pulse(Results {
            headline: Headline::Position(5),
            laps: Vec::new(),
            total_ticks: 0,
        });
        let mode = oag_race::Mode::TimeTrial;
        let cell = voice_for(pulse(), mode, &results, None, true, Some(Medal::Gold)).unwrap();
        assert_eq!(spoken(cell, Screen::Rewards, false), Some(Cue::MedalGold));
        let free = voice_for(pulse(), mode, &results, None, false, Some(Medal::Gold)).unwrap();
        assert_eq!(spoken(free, Screen::Rewards, false), None);
        let none = voice_for(None, mode, &results, None, true, Some(Medal::Gold));
        assert!(none.is_none());
    }
}
