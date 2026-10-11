//! Which voice lines a finished race's end-of-race screens speak: the place
//! and medal inputs read off the models the screens were built from, handed to
//! [`oag_game::endrace_voice::Voice`], which decides the tick.

use oag_game::endrace_voice::Voice;
use oag_ui_screens::endrace::Rewards;

use super::ResultsModel;

/// The voice for a race in `mode` whose screens hold `results` and `rewards`.
///
/// The place is the one `g_endrace_result` word 0 carries: the finishing place
/// (`race_place`), or the player's place in the Eliminator's own ranked table.
/// A Tournament speaks word 1, the player's standings rank, which is their
/// index in the standings rows. The medal counts only in a campaign cell.
/// [`None`] for Wipeout HD/Fury, whose screens were not read speaking.
#[must_use]
pub(crate) fn voice_for(
    mode: oag_race::Mode,
    results: &ResultsModel,
    race_place: Option<u8>,
    rewards: Option<&Rewards>,
) -> Option<Voice> {
    let (place, standings_place) = match results {
        ResultsModel::Hd(_) => return None,
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
    };
    let medal = rewards
        .filter(|rewards| rewards.campaign)
        .and_then(|rewards| rewards.medal);
    Some(Voice::new(mode, place, standings_place, medal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_game::endrace_voice::Screen;
    use oag_sound::sfx::Cue;
    use oag_tables::race_campaign::Medal;
    use oag_ui_screens::endrace::{
        EliminationRow, FieldResults, Headline, Results, TournamentResults, TournamentRow,
    };

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
        let voice = voice_for(oag_race::Mode::SingleRace, &results, Some(3), None).unwrap();
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
        let first = voice_for(oag_race::Mode::Eliminator, &table(5), Some(8), None).unwrap();
        assert_eq!(
            spoken(first, Screen::Results, false),
            Some(Cue::ResultsFirst)
        );
        let second = voice_for(oag_race::Mode::Eliminator, &table(1), Some(8), None).unwrap();
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
        let mut voice = voice_for(oag_race::Mode::Tournament, &results, Some(1), None).unwrap();
        assert_eq!(voice.tick(Screen::Results, false), None, "not on entry");
        assert_eq!(
            voice.tick(Screen::Results, true),
            Some(Cue::ResultsThird),
            "the standings are third, the leg first"
        );
    }

    #[test]
    fn the_medal_needs_a_campaign_cell_and_hd_speaks_nothing() {
        let results = ResultsModel::Pulse(Results {
            headline: Headline::Position(5),
            laps: Vec::new(),
            total_ticks: 0,
        });
        let rewards = |campaign| Rewards {
            medal: Some(Medal::Gold),
            campaign,
            loyalty: None,
        };
        let mode = oag_race::Mode::TimeTrial;
        let cell = voice_for(mode, &results, None, Some(&rewards(true))).unwrap();
        assert_eq!(spoken(cell, Screen::Rewards, false), Some(Cue::MedalGold));
        let free = voice_for(mode, &results, None, Some(&rewards(false))).unwrap();
        assert_eq!(spoken(free, Screen::Rewards, false), None);
        let hd = ResultsModel::Hd(FieldResults {
            headline: Headline::Position(1),
            rows: Vec::new(),
            loyalty: None,
        });
        assert!(voice_for(oag_race::Mode::SingleRace, &hd, Some(1), None).is_none());
    }
}
