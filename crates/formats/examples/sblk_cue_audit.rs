//! Dump named cues' command lists and timelines, or rank the opcodes that stop
//! a cue's timeline being walked, on one Pulse disc.
//!
//! ```sh
//! # every cue whose name is listed, in every bank that has it
//! cargo run --release -p oag-formats --example sblk_cue_audit -- cues '~SHIELD,PLASMA'
//! # the opcode census over every playing cue with a waveform
//! cargo run --release -p oag-formats --example sblk_cue_audit -- census
//! ```
//!
//! Reads `data/images/pulse-psp-eu.chd` (or the file in `OAG_AUDIT_DISC`).

use std::collections::BTreeMap;

use oag_disc::DiscImage;
use oag_formats::sblk::{self, Bank, KEY_ON_OPCODES};
use oag_formats::wad;

const ARCHIVES: [&str; 2] = ["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"];

fn banks_in(disc: &mut DiscImage, archive_path: &str) -> Vec<(String, Vec<u8>)> {
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
    for (n, entry) in dir.entries.iter().enumerate().filter(|(_, e)| e.size != 0) {
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
            out.push((format!("{archive_path}#{n}"), blob));
        }
    }
    out
}

fn every_bank() -> Vec<(String, Vec<u8>)> {
    let name = std::env::var("OAG_AUDIT_DISC").unwrap_or_else(|_| "pulse-psp-eu.chd".into());
    let mut disc = DiscImage::open(std::path::Path::new("data/images").join(&name)).expect("disc");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        out.extend(banks_in(&mut disc, archive));
    }
    out
}

fn opcodes_of(bank: &Bank, cue: &sblk::Cue) -> Vec<(u8, u32, i16)> {
    cue.range()
        .filter_map(|c| {
            let at = c * sblk::COMMAND_LEN;
            let w = bank.commands.get(at..at + sblk::COMMAND_LEN)?;
            let word = bank.order.u32(w, 0);
            Some((
                (word >> 24) as u8,
                word & 0x00ff_ffff,
                bank.order.u32(w, 4) as u16 as i16,
            ))
        })
        .collect()
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let arg = std::env::args().nth(2).unwrap_or_default();
    let banks = every_bank();
    match mode.as_str() {
        "cues" => {
            let wanted: Vec<&str> = arg.split(',').collect();
            for (label, blob) in &banks {
                let bank = Bank::parse(blob).expect("bank");
                for n in bank.sound_names() {
                    if !wanted.contains(&n.name.as_str()) {
                        continue;
                    }
                    let cue = bank.cue(n.cue).expect("cue");
                    println!(
                        "== {} in {label} (bank {}) cue {} vol {} flags {:#x}",
                        n.name, bank.name, cue.index, cue.volume, cue.flags
                    );
                    for (i, (op, operand, delay)) in opcodes_of(&bank, &cue).iter().enumerate() {
                        println!("   {i:2}: op {op:#04x} operand {operand:#08x} delay {delay}");
                    }
                    let t = bank.cue_timeline(&cue);
                    println!(
                        "   timeline: complete={} unread={:02x?} unresolved={}",
                        t.is_complete(),
                        t.unread,
                        t.unresolved
                    );
                    for g in &t.grains {
                        println!(
                            "   bend {:?} tick {:4} ({:.3}s) wf@{:#x} {:.3}s {}Hz loop={} angle {} cue_vol {} scale {:.3}",
                            bank.block.get(
                                g.sound.descriptor as usize + 8..g.sound.descriptor as usize + 10
                            ),
                            g.tick,
                            f64::from(g.tick) / sblk::timeline::TICKS_PER_SECOND,
                            g.sound.offset,
                            f64::from(g.sound.length) / 16.0 * 28.0
                                / f64::from(g.sound.sample_rate()),
                            g.sound.sample_rate(),
                            g.sound.is_looping(),
                            g.angle,
                            g.cue_volume,
                            g.scale
                        );
                    }
                }
            }
        }
        "census" => {
            let (mut total, mut complete, mut with_alt, mut blocked) = (0, 0, 0, 0);
            let mut alone: BTreeMap<u8, usize> = BTreeMap::new();
            let mut any: BTreeMap<u8, usize> = BTreeMap::new();
            let mut sets: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
            for (_, blob) in &banks {
                let bank = Bank::parse(blob).expect("bank");
                for cue in bank.cues().iter().filter(|c| c.plays()) {
                    let t = bank.cue_timeline(cue);
                    if t.grains.is_empty() && t.unread.is_empty() {
                        continue;
                    }
                    total += 1;
                    let ops: Vec<u8> = opcodes_of(&bank, cue).iter().map(|o| o.0).collect();
                    if ops.contains(&0x19) {
                        with_alt += 1;
                    }
                    if t.is_complete() {
                        complete += 1;
                        continue;
                    }
                    blocked += 1;
                    let mut u = t.unread.clone();
                    u.sort_unstable();
                    u.dedup();
                    for &o in &u {
                        *any.entry(o).or_default() += 1;
                    }
                    if u.len() == 1 {
                        *alone.entry(u[0]).or_default() += 1;
                    }
                    *sets.entry(u).or_default() += 1;
                }
            }
            println!(
                "cues reaching audio {total}: complete {complete}, blocked {blocked} (containing 0x19: {with_alt})"
            );
            let mut a: Vec<_> = any.iter().collect();
            a.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
            println!("blocked cues per opcode (any): {a:02x?}");
            let mut a: Vec<_> = alone.iter().collect();
            a.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
            println!("blocked cues whose ONLY unread opcode is X: {a:02x?}");
            let mut s: Vec<_> = sets.iter().collect();
            s.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
            for (set, n) in s.iter().take(25) {
                println!("  {n:4} unread set {set:02x?}");
            }
            let _ = KEY_ON_OPCODES;
        }
        _ => eprintln!("usage: sblk_cue_audit cues NAME,NAME | census"),
    }
}
