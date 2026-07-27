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
use oag_formats::sblk::{self, Bank};
use oag_formats::wad;

/// The PS2 music archive, relative to the disc root.
const PS2_MUSIC: &str = "54748/PS2MUSIC.WAD";

/// Bytes of each track to sample for the stereo statistics.
const PROBE_BYTES: u64 = 2 << 20;

/// The PSP archives that hold sound banks.
const PSP_ARCHIVES: [&str; 2] = ["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"];

/// Fewer banks than this means the walk stopped finding them.
const MIN_BANKS: usize = 30;

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

#[derive(Default)]
struct BankSurvey {
    blobs: usize,
    banks: usize,
    blocks: usize,
    in_spec: usize,
    defined_flags: usize,
    named: usize,
    /// Banks whose name round-trips to their own WAD entry hash.
    name_matches: usize,
    /// Of those, how many were short enough to be stored whole.
    name_candidates: usize,
    /// Mean step over RMS for each bank's decoded audio.
    roughness: Vec<f64>,
}

fn survey_banks(disc: &mut DiscImage, archive_path: &str, into: &mut BankSurvey) {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            wad::Compression::None => raw,
            wad::Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            wad::Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        into.blobs += 1;
        if !sblk::looks_like_bank(&blob) {
            continue;
        }

        let bank =
            Bank::parse(&blob).unwrap_or_else(|e| panic!("{archive_path} entry {index}: {e}"));
        into.banks += 1;
        into.blocks += bank.adpcm_blocks();
        for block in bank.waveforms.chunks_exact(sblk::ADPCM_BLOCK_LEN) {
            into.in_spec += usize::from(sblk::adpcm_block_is_in_spec(block));
            into.defined_flags += usize::from(sblk::adpcm_flag_is_defined(block));
        }

        if !bank.name.is_empty() {
            into.named += 1;
            // The field forces a NUL at byte 7, so only a name of 6 characters
            // or fewer is certainly stored whole and can be expected to hash
            // back. The longer ones are truncated and cannot.
            if bank.name.len() <= 6 {
                into.name_candidates += 1;
                let path = format!(r"Data\Sound\{}.bnk", bank.name);
                into.name_matches += usize::from(wad::hash_name(&path) == entry.name_hash);
            }
        }

        // Decoding has to produce exactly the documented sample count, which is
        // what says the block size and the samples-per-block are both right.
        let pcm = sblk::decode_adpcm(bank.waveforms);
        assert_eq!(
            pcm.len(),
            bank.adpcm_blocks() * sblk::ADPCM_BLOCK_SAMPLES,
            "{archive_path} entry {index}: wrong sample count"
        );

        // In-spec header bytes say the *framing* is PS-ADPCM; they say nothing
        // about the filters or the shift direction being right. Roughness does:
        // a wrong decode of a 4-bit differential codec is white noise, whose
        // mean step is about 1.4 times its RMS, and real audio is far below
        // that. Both are computed on the same samples.
        if pcm.len() > 1024 {
            let steps: f64 = pcm
                .windows(2)
                .map(|w| f64::from(i32::from(w[0]).abs_diff(i32::from(w[1]))))
                .sum();
            let energy: f64 = pcm.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
            #[expect(clippy::cast_precision_loss, reason = "sample counts are small")]
            let n = pcm.len() as f64;
            let rms = (energy / n).sqrt();
            let roughness = steps / (n - 1.0) / rms.max(1.0);
            into.roughness.push(roughness);
        }
    }
}

#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn every_psp_sound_bank_frames_exactly_and_holds_ps_adpcm() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    let mut survey = BankSurvey::default();
    for archive in PSP_ARCHIVES {
        survey_banks(&mut disc, archive, &mut survey);
    }

    println!("blobs          {}", survey.blobs);
    println!("banks          {}", survey.banks);
    println!("adpcm blocks   {}", survey.blocks);
    println!("in spec        {}", survey.in_spec);
    println!("defined flags  {}", survey.defined_flags);
    println!("named          {}", survey.named);
    println!(
        "name matches   {} of {} short enough to check",
        survey.name_matches, survey.name_candidates
    );
    let worst = survey.roughness.iter().copied().fold(0.0f64, f64::max);
    #[expect(clippy::cast_precision_loss, reason = "39 banks")]
    let mean = survey.roughness.iter().sum::<f64>() / survey.roughness.len() as f64;
    println!("roughness      mean {mean:.3}, worst {worst:.3} (white noise is ~1.41)");

    assert!(
        survey.banks >= MIN_BANKS,
        "only {} banks found, expected at least {MIN_BANKS}",
        survey.banks
    );
    // `Bank::parse` already refuses anything whose framing does not close, so
    // reaching here at all is the framing result. These are about the payload.
    //
    // A byte drawn at random is in spec 5/16 of the time for the predictor and
    // shift together, and its flag is one of eight values 1/32 of the time. At
    // half a million blocks, anything near 100% is only explicable as PS-ADPCM.
    assert!(
        survey.in_spec * 1000 >= survey.blocks * 999,
        "{} of {} blocks have an in-spec predictor and shift",
        survey.in_spec,
        survey.blocks
    );
    assert!(
        survey.defined_flags * 1000 >= survey.blocks * 999,
        "{} of {} blocks have a defined flag byte",
        survey.defined_flags,
        survey.blocks
    );
    // Every bank whose name survived the 7-character field resolves to its own
    // archive entry, which is what says the name field is a name.
    // Decoded audio, not noise. The bound is deliberately loose: percussion
    // and engine loops are genuinely rough, and the point is the gap to 1.41.
    assert!(
        worst < 1.0,
        "a bank decoded with a mean step of {worst:.3} times its RMS, which is noise"
    );
    assert!(survey.name_candidates >= 3);
    assert_eq!(
        survey.name_matches, survey.name_candidates,
        "a bank name did not hash back to its own entry"
    );
}
