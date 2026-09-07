//! Scratch probe for a streaming-audio decode investigation:
//! the longest race-music track on each disc, decoded through the same path
//! [`MusicFetchWorker`](oag_game::audio::MusicFetchWorker) uses
//! (`oag_game::music::load_entry`), and the process RSS delta that decode
//! actually costs.
//!
//! ```sh
//! cargo run --release -p oag-game --example audio_memory_probe
//! ```
use oag_game::music;

fn mib(bytes: f64) -> f64 {
    bytes / (1024.0 * 1024.0)
}

fn rss_mib() -> Option<f64> {
    memory_stats::memory_stats().map(|stats| mib(stats.physical_mem as f64))
}

fn report(label: &str, source: &str) {
    let Ok(Some(entries)) = music::listing(source) else {
        println!("{label}: no soundtrack read from {source}");
        return;
    };
    let Some((index, longest)) = entries
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.seconds.total_cmp(&b.seconds))
    else {
        println!("{label}: soundtrack listed but empty");
        return;
    };
    let cache_dir = std::env::temp_dir().join("oag-audio-memory-probe-cache");
    let before = rss_mib();
    let sound = match music::load_entry(source, entries[index].at, &cache_dir) {
        Ok(sound) => sound,
        Err(err) => {
            println!("{label}: decode of track {index} failed: {err:#}");
            return;
        }
    };
    let after = rss_mib();
    let buffer_mib = mib(sound.frames() as f64 * f64::from(sound.channels()) * 2.0);
    print!(
        "{label}: {} tracks, longest is track {index} at {:.1} s declared / {:.1} s decoded -> \
         Sound buffer {buffer_mib:.2} MiB ({} Hz, {} ch)",
        entries.len(),
        longest.seconds,
        f64::from(sound.seconds()),
        sound.sample_rate(),
        sound.channels()
    );
    match (before, after) {
        (Some(before), Some(after)) => println!(
            ", RSS {before:.1} -> {after:.1} MiB ({:+.2} MiB)",
            after - before
        ),
        _ => println!(", RSS unavailable on this platform"),
    }
}

fn main() {
    report("Pure PSP USA", "data/images/pure-psp-usa.chd");
    report("Pure PSP EU", "data/images/pure-psp-eu.chd");
    report("HD/Fury PS3", "data/images/hdfury-ps3-eu-dec.iso");
    // The PS2's soundtrack is loose PS2MUSIC.WAD entries rather than a
    // catalogue `music::listing` reads (`crate::audio::ps2_soundtrack` is
    // private to that module) - `docs/formats/ps2-audio.md` already measured
    // its range as 177.2-204.3 s, well under the two above, so it is not
    // reprobed here.
    println!(
        "Pulse PS2 (from docs/formats/ps2-audio.md, not reprobed): 16 tracks, longest 204.3 s -> {:.2} MiB decoded (44100 Hz stereo i16)",
        mib(204.3 * 44_100.0 * 2.0 * 2.0)
    );
}
