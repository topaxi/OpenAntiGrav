//! The pad's music controls in a race: the glue from `oag_input::music::MusicInput` to the
//! music in `oag_sound::Audio` and the song line in `Race`.
//!
//! Output only. Nothing here touches `World`, the state hash or a replay, and the caller
//! hands it input only in an unpaused, unfinished race. `docs/tools/oag-game.md`, "Music
//! controls in a race".

use oag_input::music::{MusicAction, MusicInput};
use oag_raceplay::Race;
use oag_sound::{Audio, Volume};

/// Applies one tick of `input`: a press acts once, a held up or down walks the volume line one
/// tick. Up and down held together cancel.
pub fn drive(audio: &mut Audio, race: &mut Race, music: Volume, input: MusicInput) {
    if input.pressed(MusicAction::ShowSong) {
        race.show_song_again(audio.now_playing().as_ref());
    }
    if input.pressed(MusicAction::RestartSong) {
        audio.restart_race_song();
    }
    if input.pressed(MusicAction::NextSong) {
        audio.next_race_song();
    }
    if input.pressed(MusicAction::PreviousSong) {
        audio.previous_race_song();
    }
    let step =
        i32::from(input.held(MusicAction::MusicUp)) - i32::from(input.held(MusicAction::MusicDown));
    if step != 0 {
        audio.step_music_line(step, music);
    }
}
