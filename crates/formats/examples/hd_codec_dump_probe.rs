//! Scratch probe: dump raw bytes of the largest not-PS-ADPCM waveform spans
//! from Wipeout HD's `.bnk` banks, for fingerprinting the unidentified second
//! codec `docs/formats/psp-audio.md#a-third-of-hds-waveforms-are-not-ps-adpcm`
//! names.
//!
//! ```sh
//! cargo run -q -p oag-formats --example hd_codec_dump_probe
//! ```
//!
//! Writes to `hd_codec/` under `data/` (gitignored), one `.bin` per span plus
//! an `index.txt` recording which bank and command each came from.

use std::path::Path;

use oag_formats::sblk::Bank;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: usize = 7;
const OUT_DIR: &str = concat!("data", "/scratch/hd_codec");

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

fn main() {
    let iso = Path::new(ISO);
    if !iso.exists() {
        println!("skipping: {ISO} not present");
        return;
    }
    std::fs::create_dir_all(OUT_DIR).expect("create out dir");

    let mut index = String::new();
    let mut dumped = 0;
    let mut lengths: Vec<u32> = Vec::new();
    let mut header_hits = 0usize;
    let mut header_checked = 0usize;
    let mut seen_banks = std::collections::BTreeSet::new();
    let mut by_mode: std::collections::BTreeMap<u16, (usize, usize)> =
        std::collections::BTreeMap::new();

    for (archive, path, blob) in every_bank(iso) {
        let Ok(bank) = Bank::parse(&blob) else {
            continue;
        };
        for sound in bank.sounds() {
            if sound.is_adpcm() {
                continue;
            }
            let Some(data) = bank.waveform(&sound) else {
                continue;
            };
            lengths.push(sound.length);

            // Test the header hypothesis over every non-ADPCM span, not just
            // the ones dumped to disk: a 16-byte header whose big-endian word
            // at +4 is the 16-bit sample count, i.e. `w1 * 2 + 16 == length`.
            if data.len() >= 16 {
                header_checked += 1;
                let w1 = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let hit = w1.wrapping_mul(2).wrapping_add(16) == data.len() as u32;
                if hit {
                    header_hits += 1;
                }
                let entry = by_mode.entry(sound.mode).or_insert((0, 0));
                entry.1 += 1;
                if hit {
                    entry.0 += 1;
                }
            }

            // Dump a handful of the largest spans, one per bank, for
            // fingerprinting: easier to see periodic structure in a bigger
            // sample, and one-per-bank gives cross-content diversity instead
            // of 12 near-identical takes from the same cue.
            if sound.length < 8192 || dumped >= 12 || !seen_banks.insert(path.clone()) {
                continue;
            }
            let name = format!(
                "{archive}_{}_{:05}.bin",
                path.rsplit('\\')
                    .next()
                    .unwrap_or(&path)
                    .replace(['.', '/'], "_"),
                sound.command
            );
            let out_path = Path::new(OUT_DIR).join(&name);
            std::fs::write(&out_path, data).expect("write dump");
            index.push_str(&format!(
                "{name}\t{archive}\t{path}\tcommand={}\tmode=0x{:04x}\tlen={}\n",
                sound.command, sound.mode, sound.length
            ));
            dumped += 1;
        }
    }

    println!("header hypothesis (w1*2+16==len): {header_hits} of {header_checked}");
    for (mode, (hits, total)) in &by_mode {
        println!("  mode 0x{mode:04x}: {hits} of {total}");
    }
    std::fs::write(Path::new(OUT_DIR).join("index.txt"), &index).expect("write index");
    lengths.sort_unstable();
    println!("non-adpcm spans seen: {}", lengths.len());
    println!("dumped: {dumped} to {OUT_DIR}");
    if let (Some(&min), Some(&max)) = (lengths.first(), lengths.last()) {
        println!("length range: {min}..={max}");
    }

    println!();
    println!("roughness (mean |step| / RMS), skipping the 16-byte header, big-endian i16:");
    roughness_census(iso, true);
    println!("same, with no header skip (docs' original methodology), for comparison:");
    roughness_census(iso, false);
}

/// Mean-of-per-span "roughness" (mean absolute sample-to-sample step divided
/// by the span's RMS) over every not-PS-ADPCM waveform, big-endian 16-bit.
/// White noise sits at about 1.41; real audio sits far below it. Comparable
/// to the `docs/formats/psp-audio.md` figure this probe is checking.
fn roughness_census(iso: &Path, skip_header: bool) {
    let mut per_span = Vec::new();
    for (_, _, blob) in every_bank(iso) {
        let Ok(bank) = Bank::parse(&blob) else {
            continue;
        };
        for sound in bank.sounds() {
            if sound.is_adpcm() {
                continue;
            }
            let Some(data) = bank.waveform(&sound) else {
                continue;
            };
            let body = if skip_header && data.len() > 16 {
                &data[16..]
            } else {
                data
            };
            let samples: Vec<i32> = body
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| i32::from(i16::from_be_bytes(*b)))
                .collect();
            if samples.len() < 2 {
                continue;
            }
            #[expect(clippy::cast_precision_loss, reason = "sample counts are small")]
            let n = samples.len() as f64;
            let rms = (samples
                .iter()
                .map(|&s| f64::from(s) * f64::from(s))
                .sum::<f64>()
                / n)
                .sqrt();
            if rms == 0.0 {
                continue;
            }
            let mean_step = samples
                .windows(2)
                .map(|w| f64::from((w[1] - w[0]).abs()))
                .sum::<f64>()
                / (n - 1.0);
            per_span.push(mean_step / rms);
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "span counts are small")]
    let mean = per_span.iter().sum::<f64>() / per_span.len() as f64;
    let worst = per_span.iter().cloned().fold(0.0_f64, f64::max);
    println!(
        "  span roughness  mean {mean:.3}, worst {worst:.3} over {} spans",
        per_span.len()
    );
}
