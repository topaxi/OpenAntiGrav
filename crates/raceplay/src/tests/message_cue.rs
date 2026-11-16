//! The cue a HUD message line raises as it starts to show.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;

/// A HUD message line starting to show sounds `MESSAGE` on that tick, once, and
/// not before it is raised: `Hud_UpdateMessages` plays it as a slot shows.
#[test]
fn a_message_line_showing_raises_the_message_cue_once() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    let mut race = Race::start(setup);
    race.tick(&PlayerInputs::none());
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    let count = |race: &Race| {
        race.pending_cues()
            .iter()
            .filter(|e| e.cue == Cue::Message)
            .count()
    };
    assert_eq!(count(&race), 0, "no message raised, no cue");

    race.raise_message("ER_GMA", true);
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    assert_eq!(count(&race), 1, "{:?}", race.pending_cues());
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    assert_eq!(
        count(&race),
        0,
        "the cue sounds as the line shows, not each tick"
    );
}

/// The slots that raised [`Cue::Message`] when `slot`'s board shows a line.
fn message_slots(race: &mut Race, slot: usize) -> Vec<u8> {
    race.tick(&PlayerInputs::none());
    race.drain_cues();
    race.view.huds[slot].messages.push("ER_GMA", true);
    race.tick(&PlayerInputs::none());
    race.drain_cues()
        .into_iter()
        .filter(|e| e.cue == Cue::Message)
        .map(|e| e.slot)
        .collect()
}

/// A network client seated at slot 1 hears its own board's line, raised for its
/// seat, and not slot 0's; seat 0 is the old behaviour.
#[test]
fn a_clients_message_cue_follows_its_seat() {
    use super::autopilot::two_people;
    let mut race = two_people(Some(1), true);
    assert_eq!(message_slots(&mut race, 1), vec![1]);
    let mut race = two_people(Some(1), true);
    assert_eq!(message_slots(&mut race, 0), Vec::<u8>::new());
    let mut race = two_people(Some(0), true);
    assert_eq!(message_slots(&mut race, 0), vec![0]);
    let mut race = two_people(Some(0), true);
    assert_eq!(message_slots(&mut race, 1), Vec::<u8>::new());
}

/// One machine flying both seats: every person's line sounds, once, for the
/// primary seat (`Race::heard_here`).
#[test]
fn a_machine_flying_both_seats_hears_either_message_once() {
    use super::autopilot::two_people;
    let mut race = two_people(None, false);
    assert_eq!(message_slots(&mut race, 1), vec![0]);
    let mut race = two_people(None, false);
    assert_eq!(message_slots(&mut race, 0), vec![0]);
}
