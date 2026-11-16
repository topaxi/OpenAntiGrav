//! Prints the default output's route as PipeWire reports it, for a few
//! seconds, without opening an audio device. Compare by hand with
//! `wpctl inspect @DEFAULT_AUDIO_SINK@`.

use std::time::{Duration, Instant};

use oag_audio::RouteWatcher;

fn main() {
    let seconds = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let mut watcher = RouteWatcher::new();
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        if let Some(route) = watcher.poll() {
            println!("route: {:?} {:?}", route.kind, route.device);
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}
