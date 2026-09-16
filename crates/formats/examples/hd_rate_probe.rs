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
//! **2026-09-16: HD's own PS3 SCREAM walk is decoded** (`ps3-hdfury-eu`'s
//! `Scream_KeyOnVoice` chain), so `Sound::sample_rate` now runs it for any
//! bank whose [`ByteOrder`](oag_formats::byte_order::ByteOrder) is `Big`. This
//! probe reports both the PSP-walk rate (what shipped until today) and the
//! order-driven rate (what `Sound::sample_rate` returns now) side by side, so
//! the before/after is reproducible from one command rather than quoted from
//! a doc.
//!
//! ```sh
//! cargo run -p oag-formats --example hd_rate_probe
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use oag_formats::sblk::Bank;
use oag_formats::sblk::pitch;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: usize = 7;
const STANDARD: [u32; 10] = [
    8000, 11025, 12000, 16000, 18000, 22050, 24000, 32000, 44100, 48000,
];

fn nearest_standard_rate(hz: u32) -> u32 {
    STANDARD
        .iter()
        .min_by_key(|s| (i64::from(hz) - i64::from(**s)).abs())
        .copied()
        .unwrap_or(0)
}

fn is_near_standard(hz: u32) -> bool {
    let best = nearest_standard_rate(hz);
    (f64::from(hz) - f64::from(best)).abs() / f64::from(best) <= 0.002
}

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
    let mut rates_hd = BTreeMap::new();
    let mut near_hd = 0usize;
    let mut near_psp_walk = 0usize;
    // Near-count under each walk, split by centre fine - the PSP's own
    // descriptors cluster on fine values around 65-68 (its usual ~66) and
    // fine 0/124/126 (its "48 kHz" family); HD's own natively-authored
    // descriptors would show up as a third population if there is one.
    let mut by_fine: BTreeMap<i8, (usize, usize, usize)> = BTreeMap::new(); // (count, near_hd, near_psp)
    let mut adpcm_total = 0usize;
    let mut adpcm_far = 0usize;
    let mut pcm_total = 0usize;
    let mut pcm_far = 0usize;
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

                // What Sound::sample_rate returns now (order-driven: HD's own
                // scale and base rate, since these banks are ByteOrder::Big).
                let hz_hd = sound.sample_rate();
                *rates_hd.entry(hz_hd).or_insert(0usize) += 1;
                let is_near_hd = is_near_standard(hz_hd);
                if is_near_hd {
                    near_hd += 1;
                }

                // What it would have returned under the PSP's own walk - the
                // pre-2026-09-16 behaviour, kept here only for the
                // before/after comparison.
                let psp_pitch =
                    pitch::sas_pitch(sound.centre_note, sound.centre_fine, pitch::DEFAULT_NOTE, 0);
                let hz_psp = pitch::sample_rate_hz(psp_pitch);
                let is_near_psp = is_near_standard(hz_psp);
                if is_near_psp {
                    near_psp_walk += 1;
                }

                let entry = by_fine.entry(sound.centre_fine).or_insert((0, 0, 0));
                entry.0 += 1;
                if is_near_hd {
                    entry.1 += 1;
                }
                if is_near_psp {
                    entry.2 += 1;
                }

                if sound.is_adpcm() {
                    adpcm_total += 1;
                    if !is_near_hd {
                        adpcm_far += 1;
                    }
                } else {
                    pcm_total += 1;
                    if !is_near_hd {
                        pcm_far += 1;
                    }
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
    println!("within 0.2% of a standard rate, PSP walk (before): {near_psp_walk} of {descriptors}");
    println!("within 0.2% of a standard rate, HD's own walk (now): {near_hd} of {descriptors}");
    println!("  PS-ADPCM: {adpcm_far} far of {adpcm_total} (HD walk)");
    println!("  16-bit PCM: {pcm_far} far of {pcm_total} (HD walk)");
    println!("distinct rates (HD walk): {}", rates_hd.len());
    let mut by_count: Vec<_> = rates_hd.iter().collect();
    by_count.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (hz, n) in by_count.iter().take(20) {
        println!("  {hz:6} Hz  x{n}");
    }
    let mut by_count: Vec<_> = centres.iter().collect();
    by_count.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for ((c, f), n) in by_count.iter().take(10) {
        println!("  centre {c:4} fine {f:4}  x{n}");
    }
    println!("near-count by centre fine, PSP walk vs HD walk (top 15 by descriptor count):");
    let mut by_fine_sorted: Vec<_> = by_fine.iter().collect();
    by_fine_sorted.sort_by_key(|(_, (count, ..))| std::cmp::Reverse(*count));
    for (fine, (count, near_hd, near_psp)) in by_fine_sorted.iter().take(15) {
        println!("  fine {fine:4}  x{count:5}  near-psp {near_psp:5}  near-hd {near_hd:5}");
    }
    println!(
        "centre/fine pairs that are far under both walks (candidates for a third rate source):"
    );
    let mut by_count: Vec<_> = centres.iter().collect();
    by_count.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for ((c, f), n) in by_count {
        let pitch_hd = pitch::sas_pitch_scaled(
            *c,
            *f,
            pitch::DEFAULT_NOTE,
            0,
            pitch::HD_NEGATIVE_CENTRE_SCALE,
        );
        let hz_hd = pitch::sample_rate_hz_at(pitch_hd, pitch::HD_SAMPLE_RATE);
        let pitch_psp = pitch::sas_pitch(*c, *f, pitch::DEFAULT_NOTE, 0);
        let hz_psp = pitch::sample_rate_hz(pitch_psp);
        if !is_near_standard(hz_hd) && !is_near_standard(hz_psp) {
            println!("  centre {c:4} fine {f:4}  x{n:4}  psp_hz {hz_psp:7}  hd_hz {hz_hd:7}");
        }
    }
}
