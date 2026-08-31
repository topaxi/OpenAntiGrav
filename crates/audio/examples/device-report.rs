//! Opens the default output device, plays a tone through the real mixer, and
//! reports what the callback saw.
//!
//! The one diagnostic in this workspace that needs no window and no disc: a
//! glitch that only happens with a device attached cannot be reproduced by
//! `--dump-audio`, which forces the null backend by construction. Run it, let
//! it sit for the requested seconds, and read which counter moved.
//!
//! ```sh
//! cargo run -p oag-audio --example device-report --release -- 20
//! cargo run -p oag-audio --example device-report --release -- 20 /tmp/tone.wav
//! ```
//!
//! With a path, it also records what the device was handed and writes it
//! there. That is the same `--tap-audio` the game has, on a tone this file
//! generated, so a fault in the recording can be told from a fault in the
//! game.

use std::sync::Arc;
use std::time::{Duration, Instant};

use oag_audio::{Bus, Mixer, Output, Play, Sound};

fn main() {
    let seconds: f32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10.0);

    let path = std::env::args().nth(2).map(std::path::PathBuf::from);
    let tap = path.map(|path| oag_audio::TapSpec { seconds, path });
    let output = Output::open_or_null(tap.as_ref());
    println!(
        "device: {:?}  rate: {} Hz  streaming: {}",
        output.device_name(),
        output.sample_rate(),
        output.is_streaming()
    );
    if !output.is_streaming() {
        println!("no device: nothing to measure");
        return;
    }

    // A 220 Hz sine at half scale, long enough to loop without a seam: one
    // whole number of cycles, so a click in the output is this program's fault
    // nowhere and the device path's everywhere.
    let rate = output.sample_rate();
    // One second exactly, which at 220 Hz is a whole number of cycles.
    let frames = rate as usize;
    let samples: Vec<i16> = (0..frames)
        .flat_map(|i| {
            let t = i as f32 / rate as f32;
            let s = (t * 220.0 * std::f32::consts::TAU).sin() * 0.5;
            let v = (s * f32::from(i16::MAX)) as i16;
            [v, v]
        })
        .collect();
    let tone = Arc::new(Sound::new(samples, 2, rate).expect("tone"));
    output.with_mixer(|mixer: &mut Mixer| {
        mixer.play(Play::looping(tone, Bus::Music));
    });

    let until = Instant::now() + Duration::from_secs_f32(seconds);
    let mut frame = 0u64;
    while Instant::now() < until {
        // At the frame rate the game reports at, so the throttle inside
        // `report_health` behaves the way it does in a race.
        std::thread::sleep(Duration::from_millis(16));
        output.report_health();
        output.flush_tap();
        frame += 1;
    }

    let health = output.health();
    println!(
        "after {frame} frame(s): {} dropped, {} jump(s), {} late callback(s), \
         buffer {} frames ({:.1} ms)",
        health.dropped_buffers(),
        health.jumps(),
        health.late_callbacks(),
        health.buffer_frames(),
        health.buffer_frames() as f32 * 1000.0 / output.sample_rate() as f32
    );
    let (starved, clipped) = output.with_mixer(|mixer| (mixer.starved(), mixer.clipped()));
    println!("mixer: {starved} refused, {clipped} clipped");
}
