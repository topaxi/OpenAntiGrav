//! A mod's own sounds (`docs/formats/mod-sound.md`): `play-mod-sound` queues plain events
//! beside the cues, a loop is stopped when its module faults, nothing is hashed, and the
//! `[sounds]` files load generically with a report line for each that does not.

use std::sync::Arc;

use oag_ext_abi::{Command, Slot, SoundMode};
use oag_gameplay::cue::{ModSoundEvent, ModSoundMode};

use super::extension_faults::scripted;
use super::extensions::on::prepared;
use super::extensions::{grid, run};

fn sound(slot: usize, index: u16, mode: SoundMode) -> Command {
    Command::PlayModSound {
        slot: Slot::new(slot).unwrap(),
        index,
        mode,
    }
}

fn event(index: u16, slot: u8, mode: ModSoundMode) -> ModSoundEvent {
    ModSoundEvent {
        mod_id: "test.sound".to_string(),
        index,
        slot,
        mode,
    }
}

#[test]
fn a_loop_is_queued_once_a_one_shot_every_time_and_a_stop_ends_it() {
    let mut loops = scripted(
        "test.sound",
        "",
        &[
            sound(1, 3, SoundMode::Loop),
            sound(1, 4, SoundMode::Once),
            sound(2, 3, SoundMode::Loop),
        ],
        &[],
    );
    loops.sounds = vec![("a".to_string(), "a.wav".to_string())];
    let mut race = grid(prepared(&[loops]));
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(
        race.drain_mod_sounds(),
        [
            event(3, 1, ModSoundMode::Loop),
            event(4, 1, ModSoundMode::Once),
            event(3, 2, ModSoundMode::Loop),
        ]
    );
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(
        race.drain_mod_sounds(),
        [event(4, 1, ModSoundMode::Once)],
        "a loop already running is not started again; a one-shot plays each time"
    );

    let mut stopper = scripted(
        "test.sound",
        "",
        &[sound(1, 3, SoundMode::Loop), sound(1, 3, SoundMode::Stop)],
        &[],
    );
    stopper.sounds = vec![("a".to_string(), "a.wav".to_string())];
    let mut race = grid(prepared(&[stopper]));
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(
        race.drain_mod_sounds(),
        [
            event(3, 1, ModSoundMode::Loop),
            event(3, 1, ModSoundMode::Stop)
        ]
    );
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(
        race.drain_mod_sounds().len(),
        2,
        "stopped, so the next tick starts it afresh and stops it again"
    );
}

#[test]
fn a_faulted_module_stops_the_loops_it_started() {
    let source = scripted(
        "test.sound",
        "",
        &[sound(1, 3, SoundMode::Loop), sound(2, 5, SoundMode::Loop)],
        &[],
    );
    let mut race = grid(prepared(&[source]));
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(race.drain_mod_sounds().len(), 2);
    crate::extensions::fault_for_tests(&mut race, 0);
    assert_eq!(
        race.drain_mod_sounds(),
        [
            event(3, 1, ModSoundMode::Stop),
            event(5, 2, ModSoundMode::Stop)
        ],
        "every running loop is stopped, in the order it started"
    );
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert!(
        race.drain_mod_sounds().is_empty(),
        "a faulted module is off"
    );
}

#[test]
fn a_race_that_asks_for_sounds_hashes_like_one_that_asks_for_none() {
    let quiet = scripted("test.sound", "", &[], &[]);
    let loud = scripted(
        "test.sound",
        "",
        &[sound(1, 0, SoundMode::Loop), sound(2, 1, SoundMode::Once)],
        &[],
    );
    let (_, quiet) = run(grid(prepared(&[quiet])), 120);
    let (_, loud) = run(grid(prepared(&[loud])), 120);
    assert_eq!(quiet, loud, "a sound is an output, never state");
}

#[cfg(feature = "view")]
mod loading {
    use super::*;

    fn wav() -> Vec<u8> {
        let pcm: Vec<u8> = (0..200i16).flat_map(|i| (i * 100).to_le_bytes()).collect();
        oag_formats::ps2_music::wav_with(&pcm, 22_050, 1)
    }

    #[test]
    fn a_mods_sound_files_are_loaded_and_a_bad_one_plays_nothing_and_is_named() {
        let mut source = scripted("test.sound", "", &[], &[]);
        source.sounds = ["a_good", "b_not_a_wav", "c_gone", "d_float"]
            .map(|name| (name.to_string(), format!("snd/{name}.wav")))
            .to_vec();
        let mut float = wav();
        float[20] = 3;
        source.files = Arc::new(move |path| match path {
            "snd/a_good.wav" => Ok(wav()),
            "snd/b_not_a_wav.wav" => Ok(b"definitely not a wav".to_vec()),
            "snd/d_float.wav" => Ok(float.clone()),
            other => Err(format!("{other}: not in the mod")),
        });
        let mut report = Vec::new();
        let loaded =
            crate::extension_sounds::load(&[source.clone()], "wipeout-pulse", None, &mut report);
        let good = loaded.get("test.sound", 0).expect("the good file plays");
        assert_eq!(
            (good.frames(), good.sample_rate(), good.channels()),
            (200, 22_050, 1)
        );
        for index in 1..4 {
            assert!(
                loaded.get("test.sound", index).is_none(),
                "{index} plays nothing"
            );
        }
        let said = report.join("\n");
        for name in ["sound b_not_a_wav", "sound c_gone", "sound d_float"] {
            assert!(said.contains(name), "{name} is named: {said}");
        }
        assert!(
            said.contains("RIFF") && said.contains("16-bit integer PCM"),
            "{said}"
        );
        assert!(
            said.contains("sound a_good") && said.contains("loaded"),
            "{said}"
        );
        assert!(said.contains("plays nothing"));

        let mut other = source;
        other.titles = vec!["wipeout-pulse".to_string()];
        let none = crate::extension_sounds::load(&[other], "wipeout-pure", None, &mut Vec::new());
        assert!(none.is_empty(), "another title's mod loads none");
    }
}
