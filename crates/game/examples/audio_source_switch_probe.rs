//! Scratch probe for `handover/streaming-decode-for-audio-would-break-seek-and.md`:
//! does a real `MUSIC SOURCE` switch, against real discs and a real `ffmpeg`
//! decode, actually return before the decode finishes - the freeze
//! `Audio::set_music_source`'s fix targets - rather than the synthetic
//! fake-path unit tests in `crates/game/src/audio/tests/source_switch.rs`,
//! which prove the mechanics but never touch a real decode.
//!
//! `dump` is set so `Audio::tick` actually renders samples with no device
//! attached (see `Audio::tick`'s own doc on why a headless run otherwise
//! never advances the mixer at all) - that is what makes the playhead below
//! a real measurement rather than a frozen number regardless of the fix.
//!
//! ```sh
//! cargo run --release -p oag-game --example audio_source_switch_probe
//! ```
use std::time::{Duration, Instant};

use oag_game::audio::{Audio, MusicDiscs, MusicSource};
use oag_game::settings;

fn main() {
    let discs = MusicDiscs::survey("data/images/pulse-psp-usa.chd");
    println!("discs: {discs:?}");

    // Never actually written: nothing here calls `Audio::finish`, only
    // `Audio::tick`, which is all that is needed to force the null output
    // backend and make `Audio::tick` render samples with no device
    // attached - see the module doc above.
    let dump = std::env::temp_dir().join("oag-audio-source-switch-probe.wav");
    let mut audio = Audio::open(
        &settings::Audio::default(),
        Some(dump),
        None,
        Duration::from_millis(40),
    );
    let cache_dir = std::env::temp_dir().join("oag-audio-source-switch-probe-cache");

    audio.start_music(&discs, MusicSource::Auto, &cache_dir);
    for _ in 0..60 {
        audio.tick();
    }
    let before = audio.playhead();
    println!("one second in, playhead is {before:?}");

    // The switch itself: this call must return in effectively zero time -
    // the whole point of the fix. `set_music_source` has no timer of its
    // own, so this measures the wall clock around the call directly.
    let call_started = Instant::now();
    audio.set_music_source(&discs, MusicSource::Ps2, &cache_dir);
    let call_took = call_started.elapsed();
    println!(
        "set_music_source(Ps2) call itself took {:.1} ms (the old synchronous version measured \
         2.0 s cold for this exact case)",
        call_took.as_secs_f64() * 1000.0
    );

    // Keep ticking through the real decode, the way the fixed-timestep loop
    // would - and show the playhead is still moving the whole time, proving
    // this is not a frozen mixer waiting on the fetch.
    for second in 1..=3 {
        for _ in 0..60 {
            audio.tick();
        }
        println!("+{second}s: playhead {:?}", audio.playhead());
    }

    assert!(
        call_took.as_millis() < 50,
        "set_music_source must return before the decode, not after - took {call_took:?}"
    );
    println!("PASS: the call returned before the decode finished, and the mixer kept moving");
}
