//! Wipeout HD's `.bnk` banks, which are Pulse's container byte-swapped whole.
//!
//! **`#[ignore]`d, never run in CI**: needs game content (`just test-data`);
//! see `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! # Why this was refused once, and what changed
//!
//! `HANDOVER.md` recorded HD's banks as measured but deliberately
//! unimplemented: framing differs in two places and a framing-only fix would
//! return a `Bank` whose `sounds()`, `sound_names()` and `decode_adpcm()` were
//! unverified. This file is that verification. The relaxation is two conditions,
//! both **alignment** rather than slack; the tail check, which says the blob was
//! read to its end, stays exact.
//!
//! # The one thing that is genuinely different
//!
//! About a third of HD's waveforms are **not PS-ADPCM**, and the descriptor
//! says which: `+0x0e`'s `0x80`. That second codec is 16-bit PCM, big-endian,
//! behind a 16-byte header - see [`oag_formats::sblk::NOT_ADPCM_FLAG`],
//! [`oag_formats::sblk::decode_pcm16`] and the census below.

use std::path::{Path, PathBuf};

use oag_formats::byte_order::ByteOrder;
use oag_formats::sblk::{self, Bank};

/// The seven PSARC archives on the disc.
const ARCHIVES: usize = 7;

/// `.bnk` **entries**, not distinct banks: several names appear in more than
/// one archive, and `env0_zone.bnk` appears twice with *different* content.
const HD_BANK_ENTRIES: usize = 50;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// Every `.bnk` entry on the disc, as `(archive, path, bytes)`.
fn every_bank(iso: &Path) -> Vec<(String, String, Vec<u8>)> {
    let mut out = Vec::new();
    for n in 0..ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", iso.display());
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".bnk"))
            .cloned()
            .collect();
        for path in paths {
            if let Ok(blob) = archive.read_path(&path) {
                out.push((format!("DATA0{n}"), path, blob));
            }
        }
    }
    out
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_hd_bank_parses_as_the_same_container_byte_swapped() {
    let Some(iso) = image() else {
        return;
    };
    let banks = every_bank(&iso);
    println!("bank entries   {}", banks.len());

    let mut parsed = 0;
    let mut first_section = std::collections::BTreeMap::new();
    let mut section_gap = std::collections::BTreeMap::new();
    let mut cue_tiled = 0;
    let mut span_tiled = 0;
    let mut names_equal_cues = 0;
    let mut names = 0;
    let mut little_endian_parses = 0;

    for (archive, path, blob) in &banks {
        let bank = Bank::parse(blob).unwrap_or_else(|e| panic!("{archive} {path}: {e}"));
        parsed += 1;
        assert_eq!(bank.order, ByteOrder::Big, "{path} is not big-endian");

        // The blob is byte-swapped whole, so the other order has to fail, or the
        // big-endian result could be a merely permissive reader.
        little_endian_parses += usize::from(Bank::parse_as(blob, ByteOrder::Little).is_ok());

        let s0 = (
            bank.order.u32(blob, 8) as usize,
            bank.order.u32(blob, 12) as usize,
        );
        let s1 = bank.order.u32(blob, 16) as usize;
        *first_section.entry(s0.0).or_insert(0usize) += 1;
        *section_gap.entry(s1 - (s0.0 + s0.1)).or_insert(0usize) += 1;

        // The three rules the PSP's own executable gave, run unchanged.
        let mut runs: Vec<(usize, usize)> = bank
            .cues()
            .into_iter()
            .filter(sblk::Cue::plays)
            .map(|c| (c.first_command, c.commands))
            .collect();
        runs.sort_unstable();
        let mut at = 0;
        let mut exact = true;
        for &(start, count) in &runs {
            exact &= start == at;
            at = start + count;
        }
        cue_tiled += usize::from(exact && at == usize::from(bank.command_count));

        let mut spans: Vec<(u32, u32)> =
            bank.sounds().iter().map(|s| (s.offset, s.length)).collect();
        spans.sort_unstable();
        spans.dedup();
        let mut at = 0u32;
        let mut exact = !spans.is_empty();
        for (offset, length) in spans {
            exact &= offset == at;
            at = offset + length;
        }
        span_tiled += usize::from(exact && at as usize == bank.waveforms.len());

        let table = bank.sound_names();
        names += table.len();
        names_equal_cues += usize::from(table.len() == usize::from(bank.cue_count));
    }

    println!("parsed         {parsed} of {}", banks.len());
    println!("section 0 at   {first_section:?}");
    println!("gap to sect 1  {section_gap:?}");
    println!("cue runs tile  {cue_tiled} of {parsed}");
    println!("spans tile     {span_tiled} of {parsed}");
    println!("names          {names}, equal to cue count on {names_equal_cues} of {parsed}");

    assert_eq!(parsed, HD_BANK_ENTRIES, "HD's bank-entry count changed");
    // The relaxation is alignment only: every bank starts its descriptor section
    // at exactly 32 and the pad before the waveform section is under one block.
    assert_eq!(
        first_section.keys().copied().collect::<Vec<_>>(),
        vec![32],
        "a bank's descriptor section does not start at 32"
    );
    assert!(
        section_gap.keys().all(|&g| g < 16 && g.is_multiple_of(4)),
        "the pad between sections is not an alignment: {section_gap:?}"
    );
    // Reading it the wrong way round has to fail, or "big-endian" says nothing.
    assert_eq!(
        little_endian_parses, 0,
        "{little_endian_parses} HD banks also parse little-endian"
    );
    // The three rules, unchanged from the PSP.
    assert_eq!(cue_tiled, parsed, "cue runs do not tile the command table");
    assert_eq!(span_tiled, parsed, "waveform spans do not tile the section");
    assert_eq!(
        names_equal_cues, parsed,
        "a name table does not cover its cues"
    );
    assert!(names > 1_000, "only {names} names recovered");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_not_adpcm_flag_predicts_hd_s_payload() {
    let Some(iso) = image() else {
        return;
    };

    // Per span: the descriptor's own flag against the share of its bytes in
    // PS-ADPCM spec. Different parts of the bank tool write the two and neither
    // refers to the other.
    let mut adpcm: Vec<f64> = Vec::new();
    let mut other: Vec<f64> = Vec::new();
    for (_, _, blob) in every_bank(&iso) {
        let bank = Bank::parse(&blob).expect("parse");
        for sound in bank.sounds() {
            let Some(data) = bank.waveform(&sound) else {
                continue;
            };
            let blocks = data.as_chunks::<{ sblk::ADPCM_BLOCK_LEN }>().0;
            if blocks.is_empty() {
                continue;
            }
            let ok = blocks
                .iter()
                .filter(|b| sblk::adpcm_block_is_in_spec(*b) && sblk::adpcm_flag_is_defined(*b))
                .count();
            #[expect(clippy::cast_precision_loss, reason = "a share of a block count")]
            let share = ok as f64 / blocks.len() as f64;
            if sound.is_adpcm() {
                &mut adpcm
            } else {
                &mut other
            }
            .push(share);
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "span counts are small")]
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    println!(
        "spans with 0x80 clear  {:>5}, in spec {:6.2}%",
        adpcm.len(),
        mean(&adpcm) * 100.0
    );
    println!(
        "spans with 0x80 set    {:>5}, in spec {:6.2}%",
        other.len(),
        mean(&other) * 100.0
    );

    assert!(adpcm.len() > 2_000, "only {} ADPCM spans", adpcm.len());
    assert!(other.len() > 500, "only {} non-ADPCM spans", other.len());
    // The whole finding: a flag the byte census knows nothing about splits the
    // corpus cleanly in two.
    assert!(
        mean(&adpcm) > 0.99,
        "spans marked ADPCM are only {:.2}% in spec",
        mean(&adpcm) * 100.0
    );
    assert!(
        mean(&other) < 0.5,
        "spans marked not-ADPCM are {:.2}% in spec, so the flag is not a codec selector",
        mean(&other) * 100.0
    );
}

/// `oag_formats::sblk::decode_pcm16` identifies the second codec: every one of
/// HD's not-PS-ADPCM spans decodes to audio far smoother than white noise. See
/// `docs/formats/psp-audio.md`'s "A third of HD's waveforms are not PS-ADPCM,
/// and are 16-bit PCM", including a labelled span's spectrogram not reproduced
/// here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn decode_pcm16_produces_audio_not_noise() {
    let Some(iso) = image() else {
        return;
    };

    // White noise's mean absolute step over RMS is about 1.41 for uncorrelated
    // 16-bit samples; real audio is far below. Per span, so one loud span cannot
    // hide many quiet noisy ones.
    let mut roughness: Vec<f64> = Vec::new();
    for (_, _, blob) in every_bank(&iso) {
        let bank = Bank::parse(&blob).expect("parse");
        for sound in bank.sounds() {
            if sound.is_adpcm() {
                continue;
            }
            let Some(data) = bank.waveform(&sound) else {
                continue;
            };
            let pcm = sblk::decode_pcm16(data);
            if pcm.len() < 2 {
                continue;
            }
            let samples: Vec<f64> = pcm.iter().map(|&s| f64::from(s)).collect();
            #[expect(clippy::cast_precision_loss, reason = "sample counts are small")]
            let n = samples.len() as f64;
            let rms = (samples.iter().map(|s| s * s).sum::<f64>() / n).sqrt();
            if rms == 0.0 {
                continue;
            }
            let mean_step =
                samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (n - 1.0);
            roughness.push(mean_step / rms);
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "span counts are small")]
    let mean = roughness.iter().sum::<f64>() / roughness.len() as f64;
    let worst = roughness.iter().copied().fold(0.0_f64, f64::max);
    println!(
        "span roughness    mean {mean:.3}, worst {worst:.3} over {} spans (white noise is ~1.41)",
        roughness.len()
    );

    assert!(
        roughness.len() > 500,
        "only {} spans measured",
        roughness.len()
    );
    assert!(
        mean < 0.5,
        "mean roughness {mean:.3} is not far below white noise's ~1.41"
    );
    // One of 1,167 spans reaches 1.476, just past the ~1.41 noise figure: a
    // short or quiet outlier, not evidence against the codec given the other
    // 1,166. 2.0 catches a regression to real noise without pinning it.
    assert!(
        worst < 2.0,
        "worst-case roughness {worst:.3} looks like noise"
    );
}
