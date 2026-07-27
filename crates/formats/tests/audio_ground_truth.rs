//! Validates the audio decoders against the real discs.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What these are for
//!
//! `PS2MUSIC.WAD` declares nothing about its contents - no codec, no rate, no
//! channel count - so the checks have to be arithmetic and statistical:
//!
//! - The entry chain **closes to the byte** on the archive's actual length,
//!   with no padding anywhere. That is what says the second word is the size and
//!   the third the offset rather than the other way round.
//! - Entry sizes are divisible by 4 and mostly **not** by 8, which is a 4-byte
//!   sample frame and not a larger alignment.
//! - Lag-1 against lag-2 neighbour distance says the frames are **interleaved
//!   stereo**: lag 2 is the same channel one sample on, lag 1 is the other
//!   channel at the same instant, so lag 2 has to be the smaller one. It is on
//!   every track, and it is *not* if the probe is misaligned by one sample,
//!   which is how the alignment was caught in the first place.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::ps2_music::{self, Directory};

/// The PS2 music archive, relative to the disc root.
const PS2_MUSIC: &str = "54748/PS2MUSIC.WAD";

/// Bytes of each track to sample for the stereo statistics.
const PROBE_BYTES: u64 = 2 << 20;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Mean absolute difference between samples `lag` apart.
fn neighbour_distance(samples: &[i16], lag: usize) -> f64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for i in 0..samples.len().saturating_sub(lag) {
        total += u64::from(i32::from(samples[i]).abs_diff(i32::from(samples[i + lag])));
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    #[expect(clippy::cast_precision_loss, reason = "a ratio for a threshold")]
    let mean = total as f64 / count as f64;
    mean
}

/// Pearson correlation between the two interleaved channels.
fn channel_correlation(samples: &[i16]) -> f64 {
    let left: Vec<f64> = samples.iter().step_by(2).map(|&s| f64::from(s)).collect();
    let right: Vec<f64> = samples
        .iter()
        .skip(1)
        .step_by(2)
        .map(|&s| f64::from(s))
        .collect();
    let n = left.len().min(right.len());
    if n == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "sample counts are far below 2^53"
    )]
    let len = n as f64;
    let ml = left[..n].iter().sum::<f64>() / len;
    let mr = right[..n].iter().sum::<f64>() / len;
    let mut num = 0.0;
    let mut dl = 0.0;
    let mut dr = 0.0;
    for i in 0..n {
        let a = left[i] - ml;
        let b = right[i] - mr;
        num += a * b;
        dl += a * a;
        dr += b * b;
    }
    if dl == 0.0 || dr == 0.0 {
        return 0.0;
    }
    num / (dl.sqrt() * dr.sqrt())
}

#[test]
#[ignore = "needs a PS2 disc image under data/images"]
fn the_ps2_music_archive_chains_exactly_and_holds_interleaved_stereo() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PS2_MUSIC)
        .expect("PS2MUSIC.WAD present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, ps2_music::HEADER_LEN as u64)
        .expect("header");
    let count = ps2_music::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, ps2_music::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    println!("archive      {} bytes", archive.size);
    println!("tracks       {}", dir.entries.len());
    println!("frames       {}", dir.frames());
    for (index, entry) in dir.entries.iter().enumerate() {
        println!(
            "  {index:2} {:#010x} {:10} bytes  {:6.2} s",
            entry.name_hash,
            entry.size,
            entry.seconds()
        );
    }

    // The exact one: no gaps, no padding, last entry ends on the archive.
    assert!(
        dir.chains_exactly(archive.size),
        "the entry chain does not close on the archive length"
    );

    // A 4-byte frame and nothing coarser. If everything were divisible by 8 the
    // sizes would be equally consistent with a larger block, and this would not
    // be evidence for 16-bit stereo at all.
    let by_eight = dir.entries.iter().filter(|e| e.size % 8 == 0).count();
    println!("divisible by 8: {by_eight} of {}", dir.entries.len());
    assert!(
        dir.entries.iter().all(|e| e.size % 4 == 0),
        "an entry size is not a whole number of frames"
    );
    assert!(
        by_eight < dir.entries.len(),
        "every size divisible by 8 would leave the frame size undetermined"
    );

    for (index, entry) in dir.entries.iter().enumerate() {
        // Frame-aligned, deliberately. Probing at an offset that is not a
        // multiple of 4 reads the stream one byte out and it looks like white
        // noise, which is a real failure this test would otherwise reproduce.
        let probe = u64::from(entry.offset) + (u64::from(entry.size) / 3 / 4) * 4;
        let len = PROBE_BYTES.min(u64::from(entry.size) / 3);
        let raw = disc
            .read_entry_range(&archive, probe, len)
            .expect("track bytes");
        let samples: Vec<i16> = raw
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();

        let lag1 = neighbour_distance(&samples, 1);
        let lag2 = neighbour_distance(&samples, 2);
        let correlation = channel_correlation(&samples);
        println!(
            "  {index:2} lag2/lag1 {:.3}  corr {correlation:.3}",
            lag2 / lag1
        );

        assert!(
            lag2 < lag1,
            "track {index}: lag-2 distance {lag2:.1} is not below lag-1 {lag1:.1}, which is what \
             mono or a wrong sample width looks like"
        );
        assert!(
            correlation > 0.3,
            "track {index}: channel correlation {correlation:.3} is too low for a stereo mix"
        );
    }
}
