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

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::ps2_music::{self, Directory};
use oag_formats::sblk::{self, Bank};
use oag_formats::wad;

/// The PS2 music archive, relative to the disc root.
const PS2_MUSIC: &str = "54748/PS2MUSIC.WAD";

/// The pre-race voice-over archive. Same count-first container as
/// [`PS2_MUSIC`], different payload: dual-mono rather than true stereo.
const PS2_PRERACE: &str = "54748/PRERACE.WAD";

/// Bytes of each track to sample for the stereo statistics.
const PROBE_BYTES: u64 = 2 << 20;

/// The rate `PRERACE.WAD` turns out to be recorded at. See
/// `docs/formats/ps2-voice.md`; the two tests below are the two legs of it.
const VOICE_RATE: usize = 44_100;

/// How much of each clip's head and tail to pull in to measure the pad. Has to
/// exceed [`VOICE_RATE`] frames with room to see where the speech starts.
const PAD_PROBE_FRAMES: u64 = 64 * 1024;

/// The PSP archive that carries the pre-race dialogue as ATRAC3plus.
const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

/// The PSP archives that hold sound banks.
const PSP_ARCHIVES: [&str; 2] = ["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"];

/// Fewer banks than this means the walk stopped finding them.
const MIN_BANKS: usize = 30;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
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
        let words = raw.as_chunks::<2>().0;
        let samples: Vec<i16> = words.iter().map(|c| i16::from_le_bytes(*c)).collect();

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

#[test]
#[ignore = "needs a PS2 disc image under data/images"]
fn the_ps2_prerace_archive_chains_exactly_and_holds_dual_mono() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PS2_PRERACE)
        .expect("PRERACE.WAD present")
        .clone();

    // The point of the test: the *music* reader parses this file unchanged.
    // `oag-wad` rejects it as "unknown WAD version 32" because the first word
    // is an entry count, not a version - the same false negative it gives
    // PS2MUSIC.WAD.
    let header = disc
        .read_entry_range(&archive, 0, ps2_music::HEADER_LEN as u64)
        .expect("header");
    let count = ps2_music::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, ps2_music::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    println!("archive      {} bytes", archive.size);
    println!("clips        {}", dir.entries.len());

    assert!(
        dir.chains_exactly(archive.size),
        "the entry chain does not close on the archive length"
    );
    assert!(
        dir.entries.iter().all(|e| e.size % 4 == 0),
        "an entry size is not a whole number of frames"
    );

    // What separates this archive from the music one. Both are 16-bit stereo
    // frames, but here the two channels carry identical bytes - a mono
    // recording written into a stereo stream. Asserting it keeps the two
    // archives from being conflated, and it is why the music test's
    // `correlation > 0.3` would be a meaningless check here: it is 1.0 by
    // construction.
    let mut identical = 0usize;
    for (index, entry) in dir.entries.iter().enumerate() {
        let probe = u64::from(entry.offset) + (u64::from(entry.size) / 3 / 4) * 4;
        let len = PROBE_BYTES.min(u64::from(entry.size) / 3);
        let raw = disc
            .read_entry_range(&archive, probe, len)
            .expect("clip bytes");
        let words = raw.as_chunks::<2>().0;
        let samples: Vec<i16> = words.iter().map(|c| i16::from_le_bytes(*c)).collect();

        let pairs = samples.as_chunks::<2>().0;
        let total = pairs.len();
        let same = pairs.iter().filter(|p| p[0] == p[1]).count();
        let ratio = same as f64 / total as f64;
        println!("  {index:2} {:9} bytes  L==R {ratio:.4}", entry.size);
        identical += usize::from(ratio > 0.99);
    }

    assert_eq!(
        identical,
        dir.entries.len(),
        "every clip should be dual-mono; {identical} of {} were",
        dir.entries.len()
    );
}

/// Frames of digital silence at the start of a dual-mono clip.
fn leading_silent_frames(bytes: &[u8]) -> Option<usize> {
    let frames = bytes.as_chunks::<{ ps2_music::FRAME_LEN }>().0;
    frames
        .iter()
        .position(|f| *f != [0u8; ps2_music::FRAME_LEN])
}

/// Frames of digital silence at the end of a dual-mono clip.
fn trailing_silent_frames(bytes: &[u8]) -> Option<usize> {
    let frames = bytes.as_chunks::<{ ps2_music::FRAME_LEN }>().0;
    frames
        .iter()
        .rev()
        .position(|f| *f != [0u8; ps2_music::FRAME_LEN])
}

/// The sample rate, from the PS2 side alone.
///
/// Nothing in the container declares a rate, and durations cannot settle it:
/// the 32 clips span about six seconds, so nearest-neighbour matching against
/// the PSP durations is noise - it agrees with the pairing the correlation
/// actually establishes on 9 of 32. What does settle it is that the authoring
/// tool padded every clip with **exactly one second** of digital silence at
/// each end. The shortest run at either end, over all 32 clips, is 44,100
/// frames to the frame, and no clip exceeds it by more than 80. At 48,000 Hz
/// that pad would be 0.919 s, which is not a number anyone chooses.
///
/// The other leg - that these are the same recordings the PSP disc carries -
/// cannot be tested here, because re-running the cross-correlation needs an
/// ATRAC3plus decoder this workspace does not have. See
/// `docs/formats/ps2-voice.md` for the correlation figures and how to
/// reproduce them.
#[test]
#[ignore = "needs a PS2 disc image under data/images"]
fn the_prerace_clips_are_padded_to_one_second_at_44100() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PS2_PRERACE)
        .expect("PRERACE.WAD present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, ps2_music::HEADER_LEN as u64)
        .expect("header");
    let count = ps2_music::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, ps2_music::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let probe_bytes = PAD_PROBE_FRAMES * ps2_music::FRAME_LEN as u64;
    let mut leads = Vec::with_capacity(dir.entries.len());
    let mut tails = Vec::with_capacity(dir.entries.len());

    for (index, entry) in dir.entries.iter().enumerate() {
        let head = disc
            .read_entry_range(&archive, u64::from(entry.offset), probe_bytes)
            .expect("clip head");
        let tail_at = u64::from(entry.offset) + u64::from(entry.size) - probe_bytes;
        let tail = disc
            .read_entry_range(&archive, tail_at, probe_bytes)
            .expect("clip tail");

        let lead = leading_silent_frames(&head)
            .unwrap_or_else(|| panic!("clip {index} is silent for the whole probe"));
        let trail = trailing_silent_frames(&tail)
            .unwrap_or_else(|| panic!("clip {index} is silent for the whole probe"));

        println!("  {index:2} lead {lead:6} frames  tail {trail:6} frames");
        leads.push(lead);
        tails.push(trail);
    }

    // Bounds first, so a single odd clip names itself rather than being hidden
    // in the minimum below. The slack is the speech starting or ending on a
    // zero crossing: 11 frames at the head and 80 at the tail, measured.
    for (index, &lead) in leads.iter().enumerate() {
        assert!(
            (VOICE_RATE..VOICE_RATE + 256).contains(&lead),
            "clip {index}: leading silence is {lead} frames, not a one-second pad"
        );
    }
    for (index, &trail) in tails.iter().enumerate() {
        assert!(
            (VOICE_RATE..VOICE_RATE + 512).contains(&trail),
            "clip {index}: trailing silence is {trail} frames, not a one-second pad"
        );
    }

    // The exact part. A pad that merely *covers* one second would leave the
    // minimum above 44,100; landing on it to the frame at both ends is what
    // says the tool was asked for one second at 44,100 Hz.
    assert_eq!(
        leads.iter().copied().min(),
        Some(VOICE_RATE),
        "the shortest leading pad should be exactly one second at {VOICE_RATE} Hz"
    );
    assert_eq!(
        tails.iter().copied().min(),
        Some(VOICE_RATE),
        "the shortest trailing pad should be exactly one second at {VOICE_RATE} Hz"
    );
}

/// What a RIFF header declares, for the ATRAC3plus census below.
struct RiffFormat {
    tag: u16,
    channels: u16,
    rate: u32,
    bytes_per_second: u32,
    block_align: u16,
}

/// Reads `fmt ` out of a RIFF/WAVE header.
///
/// Hand-rolled rather than pulled from a crate because this is the only place
/// in `oag-formats` that needs it, and the check has to run against bytes taken
/// straight off the disc.
fn riff_format(blob: &[u8]) -> Option<RiffFormat> {
    fn word(blob: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([blob[at], blob[at + 1], blob[at + 2], blob[at + 3]])
    }
    fn half(blob: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([blob[at], blob[at + 1]])
    }

    if blob.len() < 20 || &blob[..4] != b"RIFF" || &blob[8..12] != b"WAVE" {
        return None;
    }

    let mut pos = 12;
    while pos + 8 <= blob.len() {
        let len = usize::try_from(word(blob, pos + 4)).ok()?;
        let body = pos + 8;
        if &blob[pos..pos + 4] == b"fmt " && len >= 16 && body + 16 <= blob.len() {
            return Some(RiffFormat {
                tag: half(blob, body),
                channels: half(blob, body + 2),
                rate: word(blob, body + 4),
                bytes_per_second: word(blob, body + 8),
                block_align: half(blob, body + 12),
            });
        }
        pos = body.checked_add(len)?.checked_add(len & 1)?;
    }
    None
}

/// The PSP half of the pre-race set: 32 mono 44,100 Hz ATRAC3plus streams.
///
/// This is the other endpoint of the cross-correlation in
/// `docs/formats/ps2-voice.md`. It matters because the "32 against 32" count
/// is only evidence if the PSP population really is 32 - and nothing but the
/// RIFF headers says where it starts and stops. `Data.wad` also holds a
/// *second* ATRAC3plus population at half the bitrate, and a naive size filter
/// would have merged the two or clipped this one.
/// Surveys one PSP disc's `Data.wad` and returns the pre-race run's name hashes.
fn psp_prerace_hashes(image_name: &str, expected_first: usize) -> Option<Vec<u32>> {
    let path = image(image_name)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_DATA)
        .expect("Data.wad present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    // Every stream in the archive declares `WAVE_FORMAT_EXTENSIBLE` with the
    // ATRAC3plus subformat GUID and 44,100 Hz. Three populations sit behind
    // that, and it takes both the channel count *and* the bitrate to separate
    // them - bitrate alone pulls in 28 stereo streams scattered through the
    // archive.
    let mut voice = Vec::new();
    let mut half_rate = 0usize;
    let mut stereo = 0usize;
    for (index, entry) in dir.entries.iter().enumerate() {
        let head = disc
            .read_entry_range(&archive, u64::from(entry.offset), 128)
            .expect("blob head");
        let Some(fmt) = riff_format(&head) else {
            continue;
        };
        assert_eq!(fmt.tag, 0xfffe, "entry {index}: not WAVE_FORMAT_EXTENSIBLE");
        assert_eq!(
            usize::try_from(fmt.rate).expect("rate fits"),
            VOICE_RATE,
            "entry {index}: unexpected rate"
        );
        match (fmt.channels, fmt.bytes_per_second, fmt.block_align) {
            (1, 12_058, 560) => voice.push((index, entry.name_hash)),
            (1, 6_029, 280) => half_rate += 1,
            (2, _, _) => stereo += 1,
            other => panic!("entry {index}: unclassified stream {other:?}"),
        }
    }

    println!("{image_name}: entries {}", dir.entries.len());
    println!("  mono, 12058 B/s  {}", voice.len());
    println!("  mono, 6029 B/s   {half_rate}");
    println!("  stereo           {stereo}");
    println!("  first / last     {:?}", (voice.first(), voice.last()));

    assert_eq!(
        voice.len(),
        32,
        "{image_name}: the pre-race set should be exactly 32 streams, matching PRERACE.WAD"
    );

    // Contiguous, so the count is a population and not a scatter that happens
    // to total 32.
    let first = voice[0].0;
    assert!(
        voice.iter().enumerate().all(|(n, &(i, _))| i == first + n),
        "{image_name}: the 32 pre-race streams are not a contiguous run of entries"
    );
    assert_eq!(
        first, expected_first,
        "{image_name}: the run starts at the wrong entry"
    );

    Some(voice.into_iter().map(|(_, hash)| hash).collect())
}

#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn the_psp_prerace_streams_are_32_mono_atrac3plus_at_44100() {
    // The EU and USA builds hold different numbers of entries, so the run sits
    // one index apart; the *hashes* do not move, which is what says the two
    // builds ship the same 32 lines rather than 32 lines each.
    let eu = psp_prerace_hashes("pulse-psp-eu.chd", 867);
    let usa = psp_prerace_hashes("pulse-psp-usa.chd", 868);

    if let (Some(eu), Some(usa)) = (eu, usa) {
        assert_eq!(
            eu, usa,
            "the two PSP builds should key the pre-race clips identically"
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
        for block in bank.waveforms.as_chunks::<{ sblk::ADPCM_BLOCK_LEN }>().0 {
            into.in_spec += usize::from(sblk::adpcm_block_is_in_spec(block));
            into.defined_flags += usize::from(sblk::adpcm_flag_is_defined(block));
        }

        if !bank.name.is_empty() {
            into.named += 1;
            // The field is a hand-chosen label, not the path stem: the
            // executable gives `frontend.bnk` the label `FRNTEND` and
            // `generaltrack.bnk` the label `gentrak`, neither of which is a
            // prefix of its stem. So only the labels that happen to equal
            // their stem can hash back, and on this corpus those are exactly
            // the ones of 6 characters or fewer - a filter that holds by
            // coincidence rather than by rule. See docs/formats/psp-audio.md.
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

/// `Data\Sound\frontend.bnk`'s WAD name hash, in both `FE.wad` and `Data.wad`.
const FRONTEND_HASH: u32 = 0x75a9_1641;

/// The seven waveform spans `frontend.bnk`'s ten key-on commands resolve to.
///
/// This bank is the one the format page found from the data side, by searching
/// for a set of `(offset, length)` pairs that tile the waveform section. It is
/// asserted literally here because it is the case the rule was checked against
/// before the rule was known, so it is the one place a regression in the
/// arithmetic would be caught by a number rather than by a property.
const FRONTEND_SPANS: [(u32, u32); 7] = [
    (0, 272),
    (272, 912),
    (1184, 848),
    (2032, 2096),
    (4128, 1920),
    (6048, 8112),
    (14160, 8864),
];

/// Cue names that appear verbatim as `Sound_Play` arguments in the executable.
///
/// `"SPEEDUPPAD"` is from `docs/ghidra/functions/psp-pulse-usa/pads.md`,
/// `"~ENGINE"` from `exhaust.md` and `"ABSORB"` from `contact-response.md`.
/// None was known from the bank side, so finding each one in a decoded name
/// table is an independent confirmation that the table is being read correctly.
/// A wrong stride or a wrong base cannot produce a string the disassembler
/// found on its own.
const CUE_STRINGS: [&str; 3] = ["SPEEDUPPAD", "~ENGINE", "ABSORB"];

#[derive(Default)]
struct SoundSurvey {
    banks: usize,
    /// Commands with a key-on opcode, counted straight off the command table.
    key_on_commands: usize,
    /// Sounds [`Bank::sounds`] resolved. Below `key_on_commands` means a
    /// descriptor fell outside the descriptor section.
    sounds: usize,
    /// Distinct `(offset, length)` pairs, summed over banks.
    spans: usize,
    /// Banks that bind one waveform from more than one command.
    reuses: usize,
    /// Banks whose distinct span count equals the header's `waveform_count`.
    span_count_agrees: usize,
    /// Banks whose spans cover the waveform section with no gap.
    tiles_exactly: usize,
    /// Spans that run past the end of the waveform section, or are empty.
    outside: usize,
    /// Spans not a whole number of PS-ADPCM blocks at both ends.
    misaligned: usize,
    /// Pairs of distinct spans that overlap without one containing the other.
    partial_overlaps: usize,
    /// Pairs of distinct spans where one contains the other.
    nested: usize,
    /// Banks with [`sblk::HAS_NAME_TABLE`] clear.
    no_name_table: usize,
    /// Names recovered, summed over banks.
    names: usize,
    /// Banks whose name count equals `cue_count`.
    name_count_agrees: usize,
    /// Banks whose names index every cue exactly once.
    names_index_every_cue: usize,
    /// Names whose cue index is not a valid cue.
    bad_cue: usize,
    /// Names carrying a byte outside printable ASCII.
    unprintable: usize,
    /// Executable cue strings found in some bank's name table.
    cue_strings_found: Vec<&'static str>,
    /// Spans whose PS-ADPCM terminator sits on their last block or the one
    /// before it.
    terminated: usize,
    /// Spans carrying a terminator earlier than that, which is what a boundary
    /// in the wrong place produces.
    terminated_early: usize,
    /// Spans with no terminator at all.
    unterminated: usize,
    /// Mean sample step over RMS, per extracted span long enough to measure.
    roughness: Vec<f64>,
    /// Spans that measure as rough as white noise, named so they can be
    /// looked at rather than averaged away.
    noisy: Vec<String>,
    /// Whether `frontend.bnk` was seen and matched [`FRONTEND_SPANS`].
    frontend_matches: usize,
}

/// Walks a bank's sounds and names and folds the result into `into`.
fn survey_bank_sounds(bank: &Bank<'_>, name_hash: u32, into: &mut SoundSurvey) {
    into.banks += 1;

    let commands = bank.commands.as_chunks::<{ sblk::COMMAND_LEN }>().0;
    let key_on = commands
        .iter()
        .filter(|command| sblk::KEY_ON_OPCODES.contains(&command[3]))
        .count();
    into.key_on_commands += key_on;

    let sounds = bank.sounds();
    into.sounds += sounds.len();

    let section = bank.waveforms.len() as u64;
    for sound in &sounds {
        let end = u64::from(sound.offset) + u64::from(sound.length);
        if sound.length == 0 || end > section {
            into.outside += 1;
        }
        // A wrong base breaks this; a bank with holes in it does not. So it is
        // the check that separates "the rule is wrong" from "this bank is
        // simply not covered end to end".
        if !sound.offset.is_multiple_of(sblk::ADPCM_BLOCK_LEN as u32)
            || !sound.length.is_multiple_of(sblk::ADPCM_BLOCK_LEN as u32)
        {
            into.misaligned += 1;
        }
    }

    // Deduplicated, because a bank reusing one waveform under two commands is
    // expected - `frontend.bnk` does it three times - and identical spans are
    // not an overlap.
    let mut spans: Vec<(u32, u32)> = sounds.iter().map(|s| (s.offset, s.length)).collect();
    spans.sort_unstable();
    spans.dedup();
    into.spans += spans.len();
    if sounds.len() > spans.len() {
        into.reuses += 1;
    }
    if spans.len() == usize::from(bank.waveform_count) {
        into.span_count_agrees += 1;
    }

    let mut covered = 0u64;
    let mut reach = 0u64;
    for &(offset, length) in &spans {
        let (start, end) = (u64::from(offset), u64::from(offset) + u64::from(length));
        if start >= reach {
            covered += end - start;
        } else if end <= reach {
            into.nested += 1;
        } else {
            into.partial_overlaps += 1;
            covered += end - reach;
        }
        reach = reach.max(end);
    }
    if covered == section && section > 0 {
        into.tiles_exactly += 1;
    }

    if bank.flags & sblk::HAS_NAME_TABLE == 0 {
        into.no_name_table += 1;
    }
    let names = bank.sound_names();
    into.names += names.len();
    if names.len() == usize::from(bank.cue_count) {
        into.name_count_agrees += 1;
    }
    // Stronger than the count: the names have to index every cue exactly once.
    // Equal counts with every index in range would still allow two names on one
    // cue and another cue unnamed.
    let mut indices: Vec<u16> = names.iter().map(|entry| entry.cue).collect();
    indices.sort_unstable();
    if indices == (0..bank.cue_count).collect::<Vec<u16>>() {
        into.names_index_every_cue += 1;
    }
    for entry in &names {
        if entry.cue >= bank.cue_count {
            into.bad_cue += 1;
        }
        if !entry.name.bytes().all(|b| b.is_ascii_graphic()) {
            into.unprintable += 1;
        }
        if let Some(&found) = CUE_STRINGS.iter().find(|&&want| want == entry.name)
            && !into.cue_strings_found.contains(&found)
        {
            into.cue_strings_found.push(found);
        }
    }

    if name_hash == FRONTEND_HASH {
        println!("  frontend spans {spans:?}");
        assert_eq!(spans, FRONTEND_SPANS, "frontend.bnk's spans moved");
        assert_eq!(
            spans
                .iter()
                .map(|&(_, length)| u64::from(length))
                .sum::<u64>(),
            section,
            "frontend.bnk's spans do not sum to its waveform section"
        );
        assert_eq!(sounds.len(), 10, "frontend.bnk should bind ten waveforms");
        into.frontend_matches += 1;
    }

    // The sharpest check available, and one the command table knows nothing
    // about. PS-ADPCM carries its own terminator in each block's flag byte -
    // 1 end, 3 loop end, 5 start and end, 7 end and mute - so if these spans
    // are really where the encoder stopped, every span has one at its tail and
    // none in its body. A span boundary off by a single block would put a
    // terminator in the middle of its neighbour.
    for &(offset, length) in &spans {
        // Indexed through `get` because the span bounds are what is under test:
        // an out-of-range one is already counted above, and a panic here would
        // replace that count with an index message.
        let Some(span) = bank
            .waveforms
            .get(offset as usize..)
            .and_then(|tail| tail.get(..length as usize))
        else {
            continue;
        };
        let blocks = span.len() / sblk::ADPCM_BLOCK_LEN;
        let last = span
            .as_chunks::<{ sblk::ADPCM_BLOCK_LEN }>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, block)| matches!(block[1], 1 | 3 | 5 | 7))
            .map(|(index, _)| blocks - 1 - index)
            .max();
        match last {
            // The terminator sits on the final block or the one before it. The
            // penultimate case is the common one: the encoder flags the last
            // block it wrote and appends a block of run-out after it.
            Some(0 | 1) => into.terminated += 1,
            Some(_) => into.terminated_early += 1,
            None => into.unterminated += 1,
        }
    }

    // The point of extracting a span is that it decodes on its own. Each one
    // gets the roughness check the whole-section decode already gets: a span
    // taken at a wrong offset is out of phase with the block grid and decodes
    // to noise.
    for &(offset, length) in &spans {
        let Some(span) = bank
            .waveforms
            .get(offset as usize..)
            .and_then(|tail| tail.get(..length as usize))
        else {
            continue;
        };
        let pcm = sblk::decode_adpcm(span);
        assert_eq!(
            pcm.len(),
            span.len() / sblk::ADPCM_BLOCK_LEN * sblk::ADPCM_BLOCK_SAMPLES,
            "a span decoded to the wrong sample count"
        );
        if pcm.len() <= 1024 {
            continue;
        }
        let steps: f64 = pcm
            .windows(2)
            .map(|w| f64::from(i32::from(w[0]).abs_diff(i32::from(w[1]))))
            .sum();
        let energy: f64 = pcm.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
        #[expect(clippy::cast_precision_loss, reason = "sample counts are small")]
        let n = pcm.len() as f64;
        let rms = (energy / n).sqrt();
        let roughness = steps / (n - 1.0) / rms.max(1.0);
        if roughness >= 1.0 {
            into.noisy.push(format!(
                "{}:{offset}+{length} {roughness:.3}",
                if bank.name.is_empty() {
                    "?"
                } else {
                    &bank.name
                }
            ));
        }
        into.roughness.push(roughness);
    }

    println!(
        "  {:8} hash {name_hash:#010x} cues {:3} cmds {:4} wf {:3} | key-on {key_on:3} spans {:3} \
         covered {covered:7}/{section:7} | names {:3}",
        bank.name,
        bank.cue_count,
        bank.command_count,
        bank.waveform_count,
        spans.len(),
        names.len(),
    );
}

/// Reads every bank in one archive and folds it into `into`.
fn survey_sounds_in(disc: &mut DiscImage, archive_path: &str, into: &mut SoundSurvey) {
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

    println!("{archive_path}");
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
        if !sblk::looks_like_bank(&blob) {
            continue;
        }
        let bank =
            Bank::parse(&blob).unwrap_or_else(|e| panic!("{archive_path} entry {index}: {e}"));
        survey_bank_sounds(&bank, entry.name_hash, into);
    }
}

/// The per-sound rule, run against every bank on the disc.
///
/// `Scream_OpKeyOn` resolves `parameter_block + (command_word & 0xffffff)` to a
/// 24-byte descriptor and reads a waveform offset and length out of its last
/// two words. That is a claim about the data, so it can be checked without a
/// runtime, and the checks that discriminate are the ones a wrong base cannot
/// pass by luck:
///
/// - every span is a whole number of PS-ADPCM blocks at both ends;
/// - the distinct spans number exactly the header's `waveform_count`, a figure
///   the rule never reads;
/// - they tile the waveform section with no gap and no partial overlap.
///
/// The name table gets the same treatment: its entry count has to equal
/// `cue_count`, every recovered index has to be a valid cue, and the strings
/// the executable passes to `Sound_Play` at named call sites have to turn up in
/// it.
#[test]
#[ignore = "needs a PSP disc image under data/images"]
fn every_psp_sound_bank_splits_into_waveforms_that_tile_it() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    let mut survey = SoundSurvey::default();
    for archive in PSP_ARCHIVES {
        survey_sounds_in(&mut disc, archive, &mut survey);
    }

    let worst = survey.roughness.iter().copied().fold(0.0f64, f64::max);
    #[expect(clippy::cast_precision_loss, reason = "a few hundred spans")]
    let mean = survey.roughness.iter().sum::<f64>() / survey.roughness.len() as f64;
    println!("banks             {}", survey.banks);
    println!("key-on commands   {}", survey.key_on_commands);
    println!("sounds resolved   {}", survey.sounds);
    println!("distinct spans    {}", survey.spans);
    println!("banks reusing one {} of {}", survey.reuses, survey.banks);
    println!(
        "spans == wf count {} of {}",
        survey.span_count_agrees, survey.banks
    );
    println!(
        "tiles exactly     {} of {}",
        survey.tiles_exactly, survey.banks
    );
    println!("outside section   {}", survey.outside);
    println!("misaligned        {}", survey.misaligned);
    println!(
        "overlaps          {} partial, {} nested",
        survey.partial_overlaps, survey.nested
    );
    println!("no name table     {}", survey.no_name_table);
    println!("names             {}", survey.names);
    println!(
        "names == cues     {} of {}",
        survey.name_count_agrees, survey.banks
    );
    println!(
        "names index 0..n  {} of {}",
        survey.names_index_every_cue, survey.banks
    );
    println!(
        "bad cue / unprintable  {} / {}",
        survey.bad_cue, survey.unprintable
    );
    println!("cue strings found {:?}", survey.cue_strings_found);
    println!(
        "adpcm terminator  {} in the last two blocks, {} earlier, {} absent",
        survey.terminated, survey.terminated_early, survey.unterminated
    );
    println!(
        "span roughness    mean {mean:.3}, worst {worst:.3} over {} spans (white noise is ~1.41)",
        survey.roughness.len()
    );
    // Deduplicated on the length, because the track banks share an ambience
    // library: the same handful of source waveforms turn up in four circuits
    // apiece, so the raw count overstates how many distinct sounds are rough.
    let mut noisy_lengths: Vec<&str> = survey
        .noisy
        .iter()
        .map(|entry| entry.split('+').nth(1).unwrap_or(entry))
        .collect();
    noisy_lengths.sort_unstable();
    noisy_lengths.dedup();
    println!(
        "as rough as noise {} spans, {} distinct waveforms: {:?}",
        survey.noisy.len(),
        noisy_lengths.len(),
        survey.noisy
    );

    assert!(
        survey.banks >= MIN_BANKS,
        "only {} banks found, expected at least {MIN_BANKS}",
        survey.banks
    );
    assert_eq!(
        survey.sounds, survey.key_on_commands,
        "a key-on command's descriptor fell outside the descriptor section"
    );
    assert_eq!(
        survey.outside, 0,
        "a span is empty or runs past the section"
    );
    assert_eq!(
        survey.misaligned, 0,
        "a span is not a whole number of PS-ADPCM blocks, which is what a wrong base looks like"
    );
    assert_eq!(
        survey.partial_overlaps, 0,
        "two distinct spans overlap without one containing the other"
    );
    assert_eq!(survey.nested, 0, "one span contains another");
    assert_eq!(
        survey.span_count_agrees, survey.banks,
        "a bank's distinct spans do not number its declared waveform_count"
    );
    assert_eq!(
        survey.tiles_exactly, survey.banks,
        "a bank's spans leave part of its waveform section uncovered"
    );

    // `frontend.bnk` ships in both archives, so it should be checked twice.
    assert_eq!(
        survey.frontend_matches, 2,
        "frontend.bnk should have been seen in both FE.wad and Data.wad"
    );

    assert_eq!(
        survey.name_count_agrees, survey.banks,
        "a bank's name table does not hold one entry per cue"
    );
    assert_eq!(
        survey.bad_cue, 0,
        "a name points at a cue that does not exist"
    );
    assert_eq!(
        survey.names_index_every_cue, survey.banks,
        "a bank's names do not index every one of its cues exactly once"
    );
    assert_eq!(
        survey.unprintable, 0,
        "a recovered name is not printable ASCII"
    );
    assert_eq!(
        survey.cue_strings_found.len(),
        CUE_STRINGS.len(),
        "the executable's own cue strings should all resolve: found {:?}",
        survey.cue_strings_found
    );

    // The codec's own framing agrees with the command table's arithmetic. This
    // is the check that cannot be passed by a rule that merely partitions the
    // section plausibly: a terminator is one flag byte in eight, so putting one
    // in the tail of every span by luck is not something an off-by-a-block
    // reading does.
    assert_eq!(
        survey.terminated_early, 0,
        "a span carries a PS-ADPCM terminator in its body, so its end is in the wrong place"
    );
    assert_eq!(
        survey.unterminated, 0,
        "a span has no PS-ADPCM terminator, so it does not end where the encoder stopped"
    );

    // Every span decodes to audio on its own, not just the section as a whole.
    // The bound is on the mean rather than the worst: a handful of spans are
    // genuinely as flat as noise, because that is what they are - wind,
    // ambience, and explosions, identified by the name table above them. The
    // whole-section test's 0.408 worst is those spans averaged with the rest of
    // their bank, so per-span figures are expected to be higher, not lower.
    assert!(
        mean < 0.5,
        "the mean span decoded with a mean step of {mean:.3} times its RMS, which is noise"
    );
}
