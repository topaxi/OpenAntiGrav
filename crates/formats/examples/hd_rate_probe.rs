//! Survey what `Sound::sample_rate` reads off Wipeout HD's big-endian banks.
//!
//! The pitch decode (`sblk::pitch`) was read on the PSP and surveyed on the
//! Pulse, Pure and PS2 discs, all little-endian. HD's `klBS` banks are the
//! same container byte-swapped, and `Bank::sounds` reads the centre note and
//! fine by raw byte index (`record[0x02]`, `record[0x03]`) rather than through
//! the byte-order helper - so whether those two bytes still sit where the PSP
//! keeps them is a question this probe answers rather than assumes. Two
//! tells: the descriptor volume byte should still read `60..=127`, and the
//! rates should come out on standard values.
//!
//! ```sh
//! cargo run -p oag-formats --example hd_rate_probe
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use oag_formats::sblk::Bank;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: usize = 7;
const STANDARD: [u32; 10] = [
    8000, 11025, 12000, 16000, 18000, 22050, 24000, 32000, 44100, 48000,
];

fn main() {
    let iso = Path::new(ISO);
    if !iso.exists() {
        println!("skipping: {ISO} not present");
        return;
    }
    let mut banks = 0usize;
    let mut descriptors = 0usize;
    let mut volumes = BTreeMap::new();
    let mut centres = BTreeMap::new();
    let mut rates = BTreeMap::new();
    let mut near = 0usize;
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
            let Ok(blob) = archive.read_path(&path) else {
                continue;
            };
            let Ok(bank) = Bank::parse(&blob) else {
                continue;
            };
            banks += 1;
            let mut seen = std::collections::BTreeSet::new();
            for sound in bank.sounds() {
                if !seen.insert(sound.descriptor) {
                    continue;
                }
                descriptors += 1;
                *volumes.entry(sound.volume).or_insert(0usize) += 1;
                *centres
                    .entry((sound.centre_note, sound.centre_fine))
                    .or_insert(0usize) += 1;
                let hz = sound.sample_rate();
                *rates.entry(hz).or_insert(0usize) += 1;
                let best = STANDARD
                    .iter()
                    .min_by_key(|s| (i64::from(hz) - i64::from(**s)).abs())
                    .copied()
                    .unwrap_or(0);
                if (hz as f64 - best as f64).abs() / best as f64 <= 0.002 {
                    near += 1;
                }
            }
        }
    }
    println!("{banks} banks, {descriptors} distinct key-on descriptors");
    println!(
        "volume byte range: {:?}..={:?}",
        volumes.keys().next(),
        volumes.keys().next_back()
    );
    println!(
        "centre notes negative: {}",
        centres.keys().filter(|(c, _)| *c < 0).count()
    );
    println!(
        "centre notes positive: {}",
        centres.keys().filter(|(c, _)| *c >= 0).count()
    );
    println!("within 0.2% of a standard rate: {near} of {descriptors}");
    println!("distinct rates: {}", rates.len());
    let mut by_count: Vec<_> = rates.iter().collect();
    by_count.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (hz, n) in by_count.iter().take(20) {
        println!("  {hz:6} Hz  x{n}");
    }
    let mut by_count: Vec<_> = centres.iter().collect();
    by_count.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for ((c, f), n) in by_count.iter().take(10) {
        println!("  centre {c:4} fine {f:4}  x{n}");
    }
}
