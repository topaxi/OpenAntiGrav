//! Every RIFF-wrapped ATRAC9 file Wipeout 2048 ships, decoded in process and
//! checked against FFmpeg's independent decoder.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The census is every `.at9` entry of every 2048 archive held: both regions'
//! base packages, the v1.04 patch's two archives and the DLC packs
//! (`docs/formats/atrac9.md`, "Census"); the USA package's files are the EU
//! package's byte for byte, which is asserted rather than decoded twice. Each
//! file decodes through
//! [`oag_music::at9::decode`], the path the game takes; its `fact` chunk agrees
//! with the frames; and its PCM is within one least-significant bit of FFmpeg's,
//! negated (`docs/formats/atrac9.md`, "Conformance"). A missing `ffmpeg` skips
//! the comparison, printing why, as the movie ground truths do.
//!
//! `atrac9_against_libatrac9_sweep` is the scratch sweep that measured the
//! vendored decoder sample-identical to LibAtrac9 on every file of both titles;
//! it needs a built oracle and runs only with `OAG_SWEEP` set.

use std::path::{Path, PathBuf};

use oag_assets::psarc::Archive;
use oag_music::at9;

fn archive(path: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::exact(path)?;
    Some(Archive::open_file(&path).expect("the archive opens"))
}

/// The `.at9` entries of an archive, one shard of them in path order.
fn at9_paths(archive: &Archive, shard: usize, shards: usize) -> Vec<String> {
    archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".at9"))
        .enumerate()
        .filter(|(i, _)| i % shards == shard)
        .map(|(_, p)| p.clone())
        .collect()
}

/// What one slice of the census measured.
#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    /// `.at9` entries in the slice.
    entries: usize,
    /// Of those, RIFF-wrapped ATRAC9 by their own `fmt ` GUID.
    riff_atrac9: usize,
    /// Decoded by `at9::decode` to exactly the `fact` chunk's length.
    decoded: usize,
    /// Compared with FFmpeg sample for sample (zero without `ffmpeg`).
    compared: usize,
    /// The largest `|ours + ffmpeg|` over every compared sample.
    worst: i32,
}

/// Samples per channel in one superframe of 2048's only layout: four
/// 256-sample frames.
const SUPERFRAME_SAMPLES: u32 = 1024;

/// How many files go through one `ffmpeg` process.
const FFMPEG_BATCH: usize = 64;

fn census(path: &str, shard: usize, shards: usize) -> Option<Census> {
    let mut archive = archive(path)?;
    let ffmpeg = ffmpeg_present();
    if !ffmpeg {
        println!("skipping the ffmpeg comparison: ffmpeg is not on PATH");
    }
    let mut c = Census::default();
    let mut batch: Vec<(String, Vec<u8>, Vec<i16>, usize)> = Vec::new();
    for p in at9_paths(&archive, shard, shards) {
        c.entries += 1;
        let blob = archive.read_path(&p).expect("the entry reads");
        let Ok(header) = at9::header(&blob) else {
            println!("{p}: not RIFF ATRAC9");
            continue;
        };
        c.riff_atrac9 += 1;
        let format = header.format;
        assert_eq!(
            (format.channels, format.sample_rate, format.block_align),
            (2, 48_000, 384),
            "{p}: 2048 ships stereo 48 kHz at a 384-byte superframe"
        );
        let samples = header
            .samples
            .unwrap_or_else(|| panic!("{p}: no fact chunk"));
        assert_eq!(header.delay, 256, "{p}: the encoder delay is one frame");
        let raw = u32::try_from(header.frames.len() / 384).expect("small") * SUPERFRAME_SAMPLES;
        let pad = raw
            .checked_sub(samples + header.delay)
            .unwrap_or_else(|| panic!("{p}: fact declares more than the frames hold"));
        assert!(pad < SUPERFRAME_SAMPLES, "{p}: {pad} samples of padding");
        let pcm = at9::decode(&blob).unwrap_or_else(|e| panic!("{p}: {e:#}"));
        assert_eq!(pcm.samples.len(), samples as usize * 2, "{p}");
        c.decoded += 1;
        let skip = header.delay as usize * 2;
        if ffmpeg {
            batch.push((p, blob, pcm.samples, skip));
            if batch.len() == FFMPEG_BATCH {
                compare(&mut c, &mut batch);
            }
        }
    }
    if !batch.is_empty() {
        compare(&mut c, &mut batch);
    }
    Some(c)
}

/// Decodes a batch with one `ffmpeg` process and compares each file.
///
/// FFmpeg returns the whole decode, encoder delay and padding included, so
/// ours is compared against the slice after the delay.
fn compare(c: &mut Census, batch: &mut Vec<(String, Vec<u8>, Vec<i16>, usize)>) {
    let blobs: Vec<&[u8]> = batch.iter().map(|(_, b, _, _)| b.as_slice()).collect();
    let theirs = ffmpeg_pcm(&blobs);
    for ((p, _, ours, skip), theirs) in batch.iter().zip(theirs) {
        assert!(
            theirs.len() >= skip + ours.len(),
            "{p}: ffmpeg decodes {} samples, fewer than ours plus the delay",
            theirs.len()
        );
        let worst = ours
            .iter()
            .zip(&theirs[*skip..])
            .map(|(&a, &b)| (i32::from(a) + i32::from(b)).abs())
            .max()
            .unwrap_or(0);
        assert!(worst <= 1, "{p}: |ours + ffmpeg| reaches {worst}");
        c.worst = c.worst.max(worst);
        c.compared += 1;
    }
    batch.clear();
}

fn ffmpeg_present() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

/// FFmpeg's decode of each blob as interleaved `s16le`, one process for all.
fn ffmpeg_pcm(blobs: &[&[u8]]) -> Vec<Vec<i16>> {
    let dir = oag_testdata::scratch::dir(&format!("atrac9-gt-{:?}", std::thread::current().id()));
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-v", "error", "-xerror", "-y"]);
    for (i, blob) in blobs.iter().enumerate() {
        let input = dir.join(format!("in{i}.at9"));
        std::fs::write(&input, blob).unwrap();
        cmd.arg("-i").arg(input);
    }
    for i in 0..blobs.len() {
        cmd.args(["-map", &format!("{i}:a"), "-f", "s16le"])
            .arg(dir.join(format!("out{i}.raw")));
    }
    let out = cmd.output().expect("ffmpeg runs");
    assert!(
        out.status.success() && out.stderr.is_empty(),
        "ffmpeg: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pcm = (0..blobs.len())
        .map(|i| {
            std::fs::read(dir.join(format!("out{i}.raw")))
                .unwrap()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| i16::from_le_bytes(*b))
                .collect()
        })
        .collect();
    std::fs::remove_dir_all(&dir).unwrap();
    pcm
}

fn assert_census(path: &str, shard: usize, shards: usize, expected: Census) {
    let Some(mut c) = census(path, shard, shards) else {
        return;
    };
    if c.compared == 0 {
        // No ffmpeg: the comparison columns are not what is asserted.
        c.compared = expected.compared;
        c.worst = expected.worst;
    }
    assert_eq!(c, expected, "{path} shard {shard} of {shards}");
}

const EU_BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";
const US_BASE: &str = "data/extracted/vita/PCSA00015/base/PSP2/data.psarc";
const EU_PATCH: &str = "data/extracted/vita/PCSF00007/patch-v104/PSP2/data2.psarc";
const US_PATCH: &str = "data/extracted/vita/PCSA00015/patch-v104/PSP2/data2.psarc";

/// Every file in a slice is RIFF ATRAC9, decodes, and was compared.
const fn all(n: usize) -> Census {
    Census {
        entries: n,
        riff_atrac9: n,
        decoded: n,
        compared: n,
        worst: 1,
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn eu_base_at9_shard_0() {
    assert_census(EU_BASE, 0, 4, all(1141));
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn eu_base_at9_shard_1() {
    assert_census(EU_BASE, 1, 4, all(1140));
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn eu_base_at9_shard_2() {
    assert_census(EU_BASE, 2, 4, all(1140));
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn eu_base_at9_shard_3() {
    assert_census(EU_BASE, 3, 4, all(1140));
}

/// The USA package's `.at9` files are the EU package's, byte for byte, in the
/// base and in the v1.04 patch (measured by MD5 too, `docs/formats/atrac9.md`,
/// "Census"), so decoding them again would prove nothing new: this holds the
/// identity instead.
#[test]
#[ignore = "needs data/extracted/vita"]
fn us_at9_files_are_the_eu_ones() {
    for (us, eu, count) in [(US_BASE, EU_BASE, 4_561), (US_PATCH, EU_PATCH, 182)] {
        let (Some(mut us), Some(mut eu)) = (archive(us), archive(eu)) else {
            continue;
        };
        let us_paths = at9_paths(&us, 0, 1);
        assert_eq!(us_paths, at9_paths(&eu, 0, 1), "the same .at9 paths");
        assert_eq!(us_paths.len(), count);
        for p in &us_paths {
            assert!(
                us.read_path(p).unwrap() == eu.read_path(p).unwrap(),
                "{p} differs between the regions"
            );
        }
    }
}

/// The patch and DLC archives, one test: they are small. The USA patch's
/// `data2.psarc` is held by `us_at9_files_are_the_eu_ones`.
#[test]
#[ignore = "needs data/extracted/vita"]
fn patch_and_dlc_at9() {
    for (path, expected) in [
        (
            "data/extracted/vita/PCSF00007/patch-v104/PSP2/data1.psarc",
            0,
        ),
        (EU_PATCH, 182),
        ("data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc", 0),
        ("data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc", 0),
        (
            "data/extracted/vita/PCSA00015/patch-v104/PSP2/data1.psarc",
            0,
        ),
        ("data/extracted/vita/PCSA00015/dlc1/PSP2/dlc1.psarc", 0),
    ] {
        let expected = if expected == 0 {
            Census::default()
        } else {
            all(expected)
        };
        assert_census(path, 0, 1, expected);
    }
}

/// The scratch sweep behind `docs/formats/atrac9.md`'s LibAtrac9 numbers:
/// every RIFF ATRAC9 entry of the 2048 archives above and every ATRAC9 `.wem`
/// (loose and inside a bank) of Omega's, decoded by the vendored crate and by
/// LibAtrac9 through `scripts/atrac9-oracle.c`, compared sample for sample
/// over the whole untrimmed decode.
///
/// ```sh
/// cc -O2 -I <LibAtrac9>/C/src -o target/atrac9-oracle scripts/atrac9-oracle.c <LibAtrac9>/C/src/*.c -lm
/// OAG_SWEEP=1 OAG_ATRAC9_ORACLE=target/atrac9-oracle \
///   cargo nextest run -p oag-music --run-ignored only atrac9_against_libatrac9_sweep --no-capture
/// ```
#[test]
#[ignore = "scratch sweep: needs OAG_SWEEP, a built LibAtrac9 oracle and both titles' data"]
fn atrac9_against_libatrac9_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let oracle = PathBuf::from(std::env::var("OAG_ATRAC9_ORACLE").expect("OAG_ATRAC9_ORACLE"));
    let dir = oag_testdata::scratch::dir("atrac9-sweep");
    let mut files = 0usize;
    let mut identical = 0usize;
    let mut check =
        |label: &str, config: [u8; 4], format: oag_music::at3::Format, frames: &[u8]| {
            files += 1;
            let ours = at9::decode_superframes(&config, format, frames)
                .unwrap_or_else(|e| panic!("{label}: {e:#}"));
            let theirs = libatrac9(&oracle, &dir, config, format.block_align, frames);
            if ours == theirs {
                identical += 1;
            } else {
                println!("{label}: differs from LibAtrac9");
            }
        };
    let vita = [
        EU_BASE,
        "data/extracted/vita/PCSF00007/patch-v104/PSP2/data1.psarc",
        "data/extracted/vita/PCSF00007/patch-v104/PSP2/data2.psarc",
        "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
        "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
        US_BASE,
        "data/extracted/vita/PCSA00015/patch-v104/PSP2/data1.psarc",
        "data/extracted/vita/PCSA00015/patch-v104/PSP2/data2.psarc",
        "data/extracted/vita/PCSA00015/dlc1/PSP2/dlc1.psarc",
    ];
    for path in vita {
        let Some(mut a) = archive(path) else { continue };
        for p in at9_paths(&a, 0, 1) {
            let blob = a.read_path(&p).unwrap();
            if let Ok(h) = at9::header(&blob) {
                check(&format!("{path}:{p}"), h.config, h.format, h.frames);
            }
        }
    }
    let omega = [
        "data/extracted/ps4/omega-eu/uroot",
        "data/extracted/ps4/omega-eu-patch/uroot",
    ];
    for root in omega {
        let Some(root) = oag_testdata::exact(root) else {
            continue;
        };
        for entry in std::fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "psarc") {
                continue;
            }
            let mut a = Archive::open_file(&path).unwrap();
            for p in a.paths().to_vec() {
                let lower = p.to_ascii_lowercase();
                let mut wems: Vec<(String, Vec<u8>)> = Vec::new();
                if lower.ends_with(".wem") {
                    wems.push((p.clone(), a.read_path(&p).unwrap()));
                } else if lower.ends_with(".bnk") {
                    let blob = a.read_path(&p).unwrap();
                    if let Ok(bank) = oag_formats::wwise::Bank::parse(&blob) {
                        for m in bank.media() {
                            let bytes = bank.embedded(m.id).expect("inside DATA");
                            wems.push((format!("{p}#{}", m.id), bytes.to_vec()));
                        }
                    }
                }
                for (label, blob) in wems {
                    let Ok(wem) = oag_formats::wwise::wem::Wem::parse(&blob) else {
                        continue;
                    };
                    let f = wem.format();
                    let Some(a9) = f.atrac9() else { continue };
                    let format = oag_music::at3::Format {
                        channels: f.channels,
                        sample_rate: f.sample_rate,
                        block_align: f.block_align,
                    };
                    check(
                        &format!("{}:{label}", path.display()),
                        a9.config,
                        format,
                        wem.payload(),
                    );
                }
            }
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
    println!("{identical} of {files} ATRAC9 streams identical to LibAtrac9");
    assert_eq!(identical, files);
}

fn libatrac9(
    oracle: &Path,
    dir: &Path,
    config: [u8; 4],
    superframe: u16,
    frames: &[u8],
) -> Vec<i16> {
    let input = dir.join("in.bin");
    let output = dir.join("out.pcm");
    std::fs::write(&input, frames).unwrap();
    let status = std::process::Command::new(oracle)
        .arg(format!("{:08x}", u32::from_be_bytes(config)))
        .arg(&input)
        .arg(superframe.to_string())
        .arg(&output)
        .status()
        .expect("the oracle runs");
    assert!(status.success(), "LibAtrac9 refused a stream");
    std::fs::read(&output)
        .unwrap()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect()
}
