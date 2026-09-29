//! Census of how a cue's own key-ons relate, across every bank on every disc.
//!
//! Separates a cue with an alternate group (`0x19`), a left/right pair of one
//! waveform, and several distinct waveforms with no `0x19`, and splits the last
//! by whether [`Bank::cue_timeline`] walks it completely (only key-ons and
//! `0x05` children) or meets opcodes it does not model (guards, random delays,
//! branches). Also prints the opcode run of any cue whose name contains the
//! optional third argument.
//!
//! ```sh
//! cargo run --release -p oag-formats --example sblk_layer_census -- ~SHIELD
//! ```
//!
//! Takes about a minute: it reads every bank on five discs and HD.

use oag_disc::DiscImage;
use oag_formats::sblk::{self, Bank};
use oag_formats::wad;

const DISCS: [(&str, &[&str]); 5] = [
    (
        "pulse-psp-usa.chd",
        &["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"],
    ),
    (
        "pulse-psp-eu.chd",
        &["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"],
    ),
    ("pulse-ps2-eu.chd", &["54748/WADS2.WAD"]),
    (
        "pure-psp-usa.chd",
        &[
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
            "PSP_GAME/USRDIR/FEData.wad",
        ],
    ),
    (
        "pure-psp-eu.chd",
        &[
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
            "PSP_GAME/USRDIR/FEData.wad",
        ],
    ),
];

fn banks_in(disc: &mut DiscImage, archive_path: &str) -> Vec<Vec<u8>> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .cloned()
        .expect("archive present");
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("directory");
    let mut out = Vec::new();
    for entry in dir.entries.iter().filter(|e| e.size != 0) {
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            wad::Compression::None => raw,
            wad::Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            wad::Compression::Zlib => continue,
        };
        if sblk::looks_like_bank(&blob) {
            out.push(blob);
        }
    }
    out
}

fn every_bank() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (name, archives) in DISCS {
        let path = std::path::Path::new("data/images").join(name);
        let Ok(mut disc) = DiscImage::open(&path) else {
            continue;
        };
        for archive in archives {
            for blob in banks_in(&mut disc, archive) {
                out.push((format!("{name}:{archive}"), blob));
            }
        }
    }
    let iso = std::path::Path::new("data/images/hdfury-ps3-eu-dec.iso");
    if iso.exists() {
        for n in 0..7 {
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
                    out.push((format!("hd:DATA0{n}:{path}"), blob));
                }
            }
        }
    }
    out
}

fn main() {
    let show = std::env::args().nth(1);
    let (mut few, mut alternates, mut pairs) = (0, 0, 0);
    let (mut distinct_complete, mut distinct_unread) = (0, 0);
    for (label, blob) in every_bank() {
        let bank = Bank::parse(&blob).unwrap_or_else(|e| panic!("{label}: {e}"));
        let sounds = bank.sounds();
        let names = bank.sound_names();
        for cue in bank.cues().iter().filter(|c| c.plays()) {
            let opcodes: Vec<u8> = cue
                .range()
                .filter_map(|c| {
                    bank.commands
                        .get(c * 8..c * 8 + 4)
                        .map(|w| (bank.order.u32(w, 0) >> 24) as u8)
                })
                .collect();
            if let Some(needle) = &show
                && let Some(n) = names.iter().find(|n| n.cue == cue.index)
                && n.name.contains(needle.as_str())
            {
                println!("{label} {} opcodes {opcodes:02x?}", n.name);
            }
            let mut offsets: Vec<u32> = sounds
                .iter()
                .filter(|s| cue.range().contains(&s.command))
                .map(|s| s.offset)
                .collect();
            if offsets.len() < 2 {
                few += 1;
                continue;
            }
            if opcodes.contains(&0x19) {
                alternates += 1;
                continue;
            }
            offsets.sort_unstable();
            offsets.dedup();
            if offsets.len() == 1 {
                pairs += 1;
            } else if bank.cue_timeline(cue).is_complete() {
                distinct_complete += 1;
            } else {
                distinct_unread += 1;
            }
        }
    }
    println!(
        "fewer than two own key-ons {few}; with 0x19 {alternates}; one waveform keyed on several times, no 0x19 {pairs}; \
         distinct waveforms, no 0x19: complete timeline {distinct_complete}, unread opcodes {distinct_unread}"
    );
}
