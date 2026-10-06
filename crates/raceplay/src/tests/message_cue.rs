//! The cue a HUD message line raises as it starts to show.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

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
