//! The rows `EndRace Results` draws for the whole field, off the race's own
//! [`Board`]: one function per table that names every craft.
//!
//! In the library rather than beside the screens' runtime (the binary crate) so a
//! `--menu-page` still and a test build the very rows a finished race does.
//!
//! A race several people flew (a split screen) is told apart by
//! [`Board::persons`]: with one person a row is only *highlighted*, with several it
//! is also *named* by the seat they flew, which is **chosen, not measured**
//! (`docs/ui/split-end-race.md`).

use oag_raceplay::scoreboard::Board;
use oag_ui_screens::endrace::{
    FieldRow, Headline, LapSplit, PeopleResults, PersonName, PersonRow, Results,
};

/// Wipeout HD/Fury's whole field, ordered by place - the same feed
/// `crate::scoreboard::Overlay` draws when no `EndRace_Definition.xml` is read at
/// all. `board` is `None` only before `Race::capture_results` has run, which the
/// caller's own guard rules out, so this draws an empty field rather than failing.
#[must_use]
pub fn field_rows(board: Option<&Board>) -> Vec<FieldRow> {
    board
        .map(|board| {
            board
                .rows
                .iter()
                .map(|row| FieldRow {
                    place: row.place,
                    time_ticks: row.finish_tick,
                    player: row.player,
                    seat: (row.player && board.persons() > 1).then_some(row.slot + 1),
                    name: row.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Pulse's multiplayer table for a race several people flew: the whole field in
/// place order, each person named by their seat and every other craft by its team
/// (`roster`, the grid's team ids in slot order, resolved to the display name when
/// drawn). `None` without a board.
#[must_use]
pub fn people_rows(board: Option<&Board>, roster: &[String]) -> Option<PeopleResults> {
    let board = board?;
    Some(PeopleResults {
        rows: board
            .rows
            .iter()
            .map(|row| PersonRow {
                place: row.place,
                name: if row.player {
                    row.name
                        .clone()
                        .map_or(PersonName::Player(row.slot + 1), PersonName::Named)
                } else {
                    PersonName::Team(roster.get(usize::from(row.slot)).cloned())
                },
                time_ticks: row.finish_tick,
                person: row.player,
            })
            .collect(),
    })
}

/// One person of a two-person race, as `EndRace Results Split` and `Rewards Split` read
/// them: their craft's own laps and the numbers the loyalty law takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPerson {
    /// The grid slot they flew.
    pub slot: usize,
    /// Their place in the field, `None` where the race has no place to show.
    pub place: Option<u8>,
    /// Their craft's own splits ([`oag_race::Standing::lap_splits`]).
    pub laps: Vec<LapSplit>,
    /// Their finishing tick, or the race's last where they had not finished.
    pub total_ticks: u64,
    pub laps_completed: u32,
    pub kills: u32,
    pub zone: u16,
}

impl SplitPerson {
    /// The lap table's model, headed by `headline`.
    #[must_use]
    pub fn results(&self, headline: Headline) -> Results {
        Results {
            headline,
            laps: self.laps.clone(),
            total_ticks: self.total_ticks,
        }
    }
}

/// Each person of `board`, seat order (slot order), for a race exactly two flew.
/// `boosts` is the speedup-pad tally the race keeps for the first person's craft alone
/// ([`oag_raceplay::RunStats`]); the second person's column is left blank.
#[must_use]
pub fn split_people(
    board: &Board,
    world: &oag_gameplay::World,
    boosts: &[u32; oag_race::MAX_RECORDED_LAPS],
) -> Vec<SplitPerson> {
    let mut seats: Vec<_> = board.rows.iter().filter(|row| row.player).collect();
    seats.sort_by_key(|row| row.slot);
    seats
        .into_iter()
        .map(|row| {
            let slot = usize::from(row.slot);
            let standing = &world.ships[slot].standing;
            SplitPerson {
                slot,
                place: board.shows_place().then_some(row.place),
                laps: (0..oag_race::MAX_RECORDED_LAPS)
                    .filter_map(|index| {
                        standing.lap_splits[index].map(|ticks| LapSplit {
                            lap: (index + 1) as u32,
                            ticks,
                            boosts: (slot == 0).then_some(boosts[index]),
                        })
                    })
                    .collect(),
                total_ticks: row.finish_tick.unwrap_or(board.tick),
                laps_completed: row.laps_completed,
                kills: standing.kills,
                zone: world.race[slot].zone,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_raceplay::scoreboard::{Craft, Row, build};

    /// Eight craft; slots 0 and 1 are people and finish 5th and 7th, slot 7 is
    /// still racing when the last person crosses.
    pub(crate) fn two_people() -> Board {
        let crafts: Vec<Craft> = (0..8u8)
            .map(|slot| Craft {
                slot,
                person: slot < 2,
                place: match slot {
                    0 => 5,
                    1 => 7,
                    2 => 1,
                    3 => 2,
                    4 => 3,
                    5 => 4,
                    6 => 6,
                    _ => 8,
                },
                lap: if slot == 7 { 3 } else { 4 },
                finish_tick: (slot != 7).then(|| 7000 + u64::from(slot) * 90),
                best_lap_ticks: Some(2300 + u32::from(slot)),
            })
            .collect();
        build(&crafts, Some(3), 7630)
    }

    #[test]
    fn every_person_is_named_by_their_seat_and_highlighted() {
        let board = two_people();
        let table = people_rows(Some(&board), &[]).expect("a board");
        let people: Vec<_> = table.rows.iter().filter(|row| row.person).collect();
        assert_eq!(people.len(), 2);
        assert_eq!(people[0].name, PersonName::Player(1));
        assert_eq!(people[0].place, 5);
        assert_eq!(people[1].name, PersonName::Player(2));
        assert_eq!(people[1].place, 7);
        assert!(
            table
                .rows
                .windows(2)
                .all(|pair| pair[0].place < pair[1].place)
        );
        let racing = table.rows.last().expect("eight rows");
        assert_eq!(racing.time_ticks, None, "slot 7 had not finished");
    }

    #[test]
    fn a_network_race_names_every_person_by_the_name_they_raced_under() {
        let mut board = two_people();
        let people: Vec<usize> = (0..board.rows.len())
            .filter(|&i| board.rows[i].player)
            .collect();
        board.rows[people[0]].name = Some("Ada".into());
        let table = people_rows(Some(&board), &[]).expect("a board");
        let named: Vec<_> = table.rows.iter().filter(|row| row.person).collect();
        assert_eq!(named[0].name, PersonName::Named("Ada".into()));
        assert!(
            matches!(named[1].name, PersonName::Player(_)),
            "a person the race has no name for keeps the seat"
        );
        let rows = field_rows(Some(&board));
        assert_eq!(
            rows.iter()
                .filter_map(|row| row.name.as_deref())
                .collect::<Vec<_>>(),
            ["Ada"]
        );
    }

    #[test]
    fn hd_rows_name_a_person_only_when_there_are_several() {
        let rows = field_rows(Some(&two_people()));
        let seats: Vec<_> = rows.iter().filter_map(|row| row.seat).collect();
        assert_eq!(seats, vec![1, 2]);
        // One person: the row is highlighted and carries no seat.
        let crafts = [
            Craft {
                slot: 0,
                person: true,
                place: 1,
                lap: 4,
                finish_tick: Some(7000),
                best_lap_ticks: None,
            },
            Craft {
                slot: 1,
                person: false,
                place: 2,
                lap: 4,
                finish_tick: Some(7100),
                best_lap_ticks: None,
            },
        ];
        let alone = build(&crafts, Some(3), 7000);
        assert!(
            field_rows(Some(&alone))
                .iter()
                .all(|row| row.seat.is_none())
        );
        assert!(field_rows(Some(&alone))[0].player);
    }

    /// `field_rows` reads place/finish tick/player straight off the
    /// field's own `Board`, in the board's own order - no reordering, no
    /// renumbering, the same "the places are used exactly as assigned"
    /// rule `Race::capture_results`'s own doc states for the board itself.
    /// A craft still racing when the board was taken (`finish_tick: None`)
    /// keeps that absence rather than a guessed time.
    #[test]
    fn field_rows_reads_place_finish_tick_and_player_off_the_board_unchanged() {
        let board = Board {
            rows: vec![
                Row {
                    place: 1,
                    slot: 2,
                    laps_completed: 3,
                    finish_tick: Some(1234),
                    best_lap_ticks: Some(400),
                    player: false,
                    name: None,
                },
                Row {
                    place: 2,
                    slot: 0,
                    laps_completed: 2,
                    finish_tick: None,
                    best_lap_ticks: Some(410),
                    player: true,
                    name: None,
                },
            ],
            laps_target: Some(3),
            tick: 1234,
        };
        let rows = field_rows(Some(&board));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].place, 1);
        assert_eq!(rows[0].time_ticks, Some(1234));
        assert!(!rows[0].player);
        assert_eq!(rows[1].place, 2);
        assert_eq!(rows[1].time_ticks, None);
        assert!(rows[1].player);
    }

    #[test]
    fn field_rows_is_empty_rather_than_panicking_with_no_board() {
        assert!(field_rows(None).is_empty());
    }
}
